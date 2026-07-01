use crate::cache::CacheStatus::{Cached, NonCached};
use crate::cache::{
	CACHE_KEY_VERSION, CACHE_PREFIX, CacheStatus, deserialize_option_redis_value,
	serialize_option_redis_value, spawn_cache_write,
};
use crate::db::game::{
	find_all_games_and_id_mappings_by_crc, find_all_games_and_id_mappings_by_md5,
	find_all_games_and_id_mappings_by_sha1, find_all_games_and_id_mappings_by_sha256,
	find_game_and_id_mapping_by_crc, find_game_and_id_mapping_by_md5,
	find_game_and_id_mapping_by_name_and_size, find_game_and_id_mapping_by_sha1,
	find_game_and_id_mapping_by_sha256, find_game_ids_sharing_hash,
};
use crate::db::game_file::{get_game_files_from_game_id, get_game_files_from_game_ids};
use crate::error::ServiceResult;
use crate::model::GameMatchType;
use entity::{game, game_file, signature_metadata_mapping};
use hex::encode as hex_encode;
use log::{debug, warn};
use redis::aio::MultiplexedConnection;
use redis::{AsyncTypedCommands, Expiry};
use sea_orm::DbConn;
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::time::Duration;

const IDENTIFY_CACHE_LIFETIME: u64 = Duration::from_secs(60 * 60 * 24 * 7).as_secs();

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IdentifyEntry {
	pub game: game::Model,
	pub metadata_mappings: Vec<signature_metadata_mapping::Model>,
}

/// Ranked V2 identify payload: every distinct co-hashed game with its mappings,
/// ordered by the same spine the single-winner resolver uses. Element zero is
/// the V1 winner; ranks one and on are the surfaced siblings.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IdentifyEntryV2 {
	pub games: Vec<IdentifyEntry>,
}

/// Render the redis key for an identify cache entry. Panics on
/// [`GameMatchType::NoMatch`] (not a cacheable match type); callers
/// should only pass match types whose
/// [`GameMatchType::cache_segment`] returns `Some`.
fn identify_cache_key(match_type: GameMatchType, identifier: &str) -> String {
	let segment = match_type
		.cache_segment()
		.expect("identify_cache_key called with non-cacheable match type");
	format!("{CACHE_PREFIX}:cache:{CACHE_KEY_VERSION}:identify:{segment}:{identifier}")
}

/// V2 ranked-union identify key. Separate namespace from
/// [`identify_cache_key`] so the V1 payload shape stays untouched while the V2
/// read path caches the full ranked sibling set under the same identifier.
fn identify_v2_cache_key(match_type: GameMatchType, identifier: &str) -> String {
	let segment = match_type
		.cache_segment()
		.expect("identify_v2_cache_key called with non-cacheable match type");
	format!("{CACHE_PREFIX}:cache:{CACHE_KEY_VERSION}:identify_v2:{segment}:{identifier}")
}

/// Produce the opaque identifier segment used in the filename+size identify
/// cache key. Null-byte separator guarantees `("a", 12)` and `("a1", 2)` do
/// not collide, and lowercasing the filename matches the case-insensitive
/// database lookup.
pub fn filename_size_key(file_name: &str, file_size: i64) -> String {
	let mut hasher = Sha256::new();
	hasher.update(file_name.to_lowercase().as_bytes());
	hasher.update(b"\0");
	hasher.update(file_size.to_string().as_bytes());
	hex_encode(hasher.finalize())
}

/// Identifier segment for the CRC+size identify cache key. The CRC rung now
/// matches on crc and size together, so the cache key must carry both or two
/// same-CRC/different-size files would collide on one Redis entry. The crc is
/// hex and the size is decimal, so a single separator is unambiguous; lowercasing
/// the crc matches the case-insensitive database lookup.
pub fn crc_size_key(crc: &str, file_size: i64) -> String {
	format!("{}:{file_size}", crc.to_lowercase())
}

/// Push every identify cache key derived from `file` (sha256, sha1, md5,
/// filename+size) into `out`. Skips fields that are missing on the model so
/// the caller can pipeline a single `DEL` over only the keys that exist.
pub fn collect_identify_cache_keys(file: &game_file::Model, out: &mut Vec<String>) {
	if let Some(sha256) = &file.sha256 {
		out.push(identify_cache_key(GameMatchType::SHA256, sha256));
	}
	if let Some(sha1) = &file.sha1 {
		out.push(identify_cache_key(GameMatchType::SHA1, sha1));
	}
	if let Some(md5) = &file.md5 {
		out.push(identify_cache_key(GameMatchType::MD5, md5));
	}
	// The CRC rung is keyed on crc+size, so the bust key must carry both. A file
	// with a crc but no recorded size never produces a CRC identify entry (the
	// read path always has a size), so nothing to bust in that case.
	if let (Some(crc), Some(size)) = (&file.crc, file.file_size_in_bytes) {
		out.push(identify_cache_key(
			GameMatchType::CRC,
			&crc_size_key(crc, size),
		));
	}
	if let Some(size) = file.file_size_in_bytes {
		let key = filename_size_key(&file.file_name, size);
		out.push(identify_cache_key(GameMatchType::FileNameAndSize, &key));
	}
}

/// Bust every identify cache key derived from the game's files. Soft
/// consistency: a cache miss already in flight at the moment of this call can
/// still write stale data after the `DEL` completes; the next mapping write or
/// the 7-day TTL eventually heals it.
pub async fn bust_identify_cache_for_game(
	redis_conn: &mut MultiplexedConnection,
	db_conn: &DbConn,
	game_id: Uuid,
) -> ServiceResult<()> {
	let files = get_game_files_from_game_id(game_id, db_conn).await?;
	let mut keys: Vec<String> = Vec::new();
	for file in &files {
		collect_identify_cache_keys(file, &mut keys);
	}
	if keys.is_empty() {
		return Ok(());
	}
	if let Err(e) = redis_conn.del(&keys).await {
		warn!(
			"identify cache bust failed for game {game_id} ({} keys): {e}",
			keys.len()
		);
	}
	Ok(())
}

/// Bust both the V1 and V2 identify namespaces for every game that shares a
/// content hash with any of `files`. A pure-addition import fires no lifecycle
/// retirement, so this is the only path that heals a cached winner whose ranked
/// sibling set changed when a co-hashed game was added.
///
/// For each hash on each file the union of co-hashed `game_id`s is resolved
/// through an index-backed `LOWER(<col>)` lookup, then every V1 and V2 key for
/// every resolved game is deleted in one pipeline.
pub async fn bust_identify_cache_for_hashes(
	redis_conn: &mut MultiplexedConnection,
	db_conn: &DbConn,
	files: &[game_file::Model],
) -> ServiceResult<()> {
	let mut game_ids: HashSet<Uuid> = HashSet::new();
	for file in files {
		for (match_type, value) in [
			(GameMatchType::SHA256, &file.sha256),
			(GameMatchType::SHA1, &file.sha1),
			(GameMatchType::MD5, &file.md5),
			(GameMatchType::CRC, &file.crc),
		] {
			if let Some(hash) = value {
				for id in find_game_ids_sharing_hash(match_type, hash, db_conn).await? {
					game_ids.insert(id);
				}
			}
		}
	}

	if game_ids.is_empty() {
		return Ok(());
	}

	let mut keys: Vec<String> = Vec::new();
	let game_ids: Vec<Uuid> = game_ids.into_iter().collect();
	for file in &get_game_files_from_game_ids(&game_ids, db_conn).await? {
		collect_identify_cache_keys(file, &mut keys);
		collect_identify_v2_cache_keys(file, &mut keys);
	}

	if keys.is_empty() {
		return Ok(());
	}
	if let Err(e) = redis_conn.del(&keys).await {
		warn!(
			"hash-aware identify cache bust failed ({} keys): {e}",
			keys.len()
		);
	}
	Ok(())
}

/// V2 counterpart to [`collect_identify_cache_keys`]: the ranked-union keys for
/// every hash on `file`. Filename+size is not a ranked V2 segment, so it is not
/// emitted here.
fn collect_identify_v2_cache_keys(file: &game_file::Model, out: &mut Vec<String>) {
	if let Some(sha256) = &file.sha256 {
		out.push(identify_v2_cache_key(GameMatchType::SHA256, sha256));
	}
	if let Some(sha1) = &file.sha1 {
		out.push(identify_v2_cache_key(GameMatchType::SHA1, sha1));
	}
	if let Some(md5) = &file.md5 {
		out.push(identify_v2_cache_key(GameMatchType::MD5, md5));
	}
	if let (Some(crc), Some(size)) = (&file.crc, file.file_size_in_bytes) {
		out.push(identify_v2_cache_key(
			GameMatchType::CRC,
			&crc_size_key(crc, size),
		));
	}
}

/// Ranked V2 sibling resolver behind the cache. Reads the `identify_v2`
/// namespace; on a miss runs the ranked DB resolver for `match_type` and writes
/// the full union back. Only the four hash match types are ranked here.
pub async fn find_all_games_and_metadata_ids_by_hash_cached(
	hash: &str,
	match_type: GameMatchType,
	redis_conn: &mut MultiplexedConnection,
	db_conn: &DbConn,
) -> ServiceResult<CacheStatus<IdentifyEntryV2>> {
	let cache_key = identify_v2_cache_key(match_type, hash);
	let metric_segment = match_type
		.cache_segment()
		.expect("v2 hash cache called with non-cacheable match type");

	if let Ok(Some(cached_val)) = redis_conn
		.get_ex(&cache_key, Expiry::EX(IDENTIFY_CACHE_LIFETIME))
		.await
	{
		debug!("V2 cache hit for key: {hash}");
		crate::metrics::record_cache_hit("identify_v2", metric_segment);
		let deserialized: IdentifyEntryV2 = serde_json::from_str(&cached_val)?;
		return Ok(Cached(deserialized));
	}

	debug!("V2 cache miss for key: {hash}");
	crate::metrics::record_cache_miss("identify_v2", metric_segment);

	let ranked = match match_type {
		GameMatchType::SHA256 => find_all_games_and_id_mappings_by_sha256(hash, db_conn).await?,
		GameMatchType::SHA1 => find_all_games_and_id_mappings_by_sha1(hash, db_conn).await?,
		GameMatchType::MD5 => find_all_games_and_id_mappings_by_md5(hash, db_conn).await?,
		GameMatchType::CRC => {
			unreachable!("crc is ranked through find_all_games_and_metadata_ids_by_crc_size_cached")
		}
		GameMatchType::FileNameAndSize | GameMatchType::NoMatch => {
			unreachable!("only hash match types are ranked for V2")
		}
	};

	let entry = IdentifyEntryV2 {
		games: ranked
			.into_iter()
			.map(|(game, metadata_mappings)| IdentifyEntry {
				game,
				metadata_mappings,
			})
			.collect(),
	};

	let payload = serde_json::to_string(&entry)?;
	spawn_cache_write(
		redis_conn.clone(),
		cache_key,
		payload,
		IDENTIFY_CACHE_LIFETIME,
	);

	Ok(NonCached(entry))
}

/// Ranked V2 sibling resolver for the CRC rung, keyed on crc+size. The CRC match
/// type is pinned to an exact size, so it cannot share the generic hash resolver
/// (which keys on the hash alone); it mirrors that resolver otherwise.
pub async fn find_all_games_and_metadata_ids_by_crc_size_cached(
	crc: &str,
	file_size: i64,
	redis_conn: &mut MultiplexedConnection,
	db_conn: &DbConn,
) -> ServiceResult<CacheStatus<IdentifyEntryV2>> {
	let cache_key = identify_v2_cache_key(GameMatchType::CRC, &crc_size_key(crc, file_size));
	let metric_segment = GameMatchType::CRC
		.cache_segment()
		.expect("CRC is cacheable");

	if let Ok(Some(cached_val)) = redis_conn
		.get_ex(&cache_key, Expiry::EX(IDENTIFY_CACHE_LIFETIME))
		.await
	{
		debug!("V2 cache hit for crc+size");
		crate::metrics::record_cache_hit("identify_v2", metric_segment);
		let deserialized: IdentifyEntryV2 = serde_json::from_str(&cached_val)?;
		return Ok(Cached(deserialized));
	}

	debug!("V2 cache miss for crc+size");
	crate::metrics::record_cache_miss("identify_v2", metric_segment);

	let ranked = find_all_games_and_id_mappings_by_crc(crc, file_size, db_conn).await?;
	let entry = IdentifyEntryV2 {
		games: ranked
			.into_iter()
			.map(|(game, metadata_mappings)| IdentifyEntry {
				game,
				metadata_mappings,
			})
			.collect(),
	};

	let payload = serde_json::to_string(&entry)?;
	spawn_cache_write(
		redis_conn.clone(),
		cache_key,
		payload,
		IDENTIFY_CACHE_LIFETIME,
	);

	Ok(NonCached(entry))
}

pub async fn find_game_and_metadata_ids_by_filename_size_cached(
	file_name: &str,
	file_size: i64,
	redis_conn: &mut MultiplexedConnection,
	db_conn: &DbConn,
) -> ServiceResult<CacheStatus<Option<IdentifyEntry>>> {
	let key = filename_size_key(file_name, file_size);
	let cache_key = identify_cache_key(GameMatchType::FileNameAndSize, &key);
	let metric_segment = GameMatchType::FileNameAndSize
		.cache_segment()
		.expect("FileNameAndSize is cacheable");

	if let Ok(Some(cached_val)) = redis_conn
		.get_ex(&cache_key, Expiry::EX(IDENTIFY_CACHE_LIFETIME))
		.await
	{
		debug!("Cache hit for filename+size");
		crate::metrics::record_cache_hit("identify", metric_segment);
		let deserialized = deserialize_option_redis_value(cached_val)?;
		return Ok(Cached(deserialized));
	}

	debug!("Cache miss for filename+size");
	crate::metrics::record_cache_miss("identify", metric_segment);

	let entry = find_game_and_id_mapping_by_name_and_size(file_name, file_size, db_conn)
		.await?
		.map(|(game, mappings)| IdentifyEntry {
			game,
			metadata_mappings: mappings,
		});

	let payload = serialize_option_redis_value(entry.clone())?;
	spawn_cache_write(
		redis_conn.clone(),
		cache_key,
		payload,
		IDENTIFY_CACHE_LIFETIME,
	);

	Ok(NonCached(entry))
}

/// Single-winner resolver for the CRC rung, keyed on crc+size. CRC is pinned to
/// an exact size, so it has its own cached wrapper rather than going through the
/// hash-only resolver. Mirrors [`find_game_and_metadata_ids_by_filename_size_cached`].
pub async fn find_game_and_metadata_ids_by_crc_size_cached(
	crc: &str,
	file_size: i64,
	redis_conn: &mut MultiplexedConnection,
	db_conn: &DbConn,
) -> ServiceResult<CacheStatus<Option<IdentifyEntry>>> {
	let cache_key = identify_cache_key(GameMatchType::CRC, &crc_size_key(crc, file_size));
	let metric_segment = GameMatchType::CRC
		.cache_segment()
		.expect("CRC is cacheable");

	if let Ok(Some(cached_val)) = redis_conn
		.get_ex(&cache_key, Expiry::EX(IDENTIFY_CACHE_LIFETIME))
		.await
	{
		debug!("Cache hit for crc+size");
		crate::metrics::record_cache_hit("identify", metric_segment);
		let deserialized = deserialize_option_redis_value(cached_val)?;
		return Ok(Cached(deserialized));
	}

	debug!("Cache miss for crc+size");
	crate::metrics::record_cache_miss("identify", metric_segment);

	let entry = find_game_and_id_mapping_by_crc(crc, file_size, db_conn)
		.await?
		.map(|(game, mappings)| IdentifyEntry {
			game,
			metadata_mappings: mappings,
		});

	let payload = serialize_option_redis_value(entry.clone())?;
	spawn_cache_write(
		redis_conn.clone(),
		cache_key,
		payload,
		IDENTIFY_CACHE_LIFETIME,
	);

	Ok(NonCached(entry))
}

pub async fn find_game_and_metadata_ids_by_hash_cached(
	hash: &str,
	match_type: GameMatchType,
	redis_conn: &mut MultiplexedConnection,
	db_conn: &DbConn,
) -> ServiceResult<CacheStatus<Option<IdentifyEntry>>> {
	let cache_key = identify_cache_key(match_type, hash);
	let metric_segment = match_type
		.cache_segment()
		.expect("hash cache called with non-cacheable match type");

	if let Ok(Some(cached_val)) = redis_conn
		.get_ex(&cache_key, Expiry::EX(IDENTIFY_CACHE_LIFETIME))
		.await
	{
		debug!("Cache hit for key: {hash}");
		crate::metrics::record_cache_hit("identify", metric_segment);
		let deserialized = deserialize_option_redis_value(cached_val)?;
		return Ok(Cached(deserialized));
	}

	debug!("Cache miss for key: {hash}");
	crate::metrics::record_cache_miss("identify", metric_segment);

	let entry = match match_type {
		GameMatchType::SHA256 => find_game_and_id_mapping_by_sha256(hash, db_conn).await?,
		GameMatchType::SHA1 => find_game_and_id_mapping_by_sha1(hash, db_conn).await?,
		GameMatchType::MD5 => find_game_and_id_mapping_by_md5(hash, db_conn).await?,
		GameMatchType::CRC => {
			unreachable!("crc has a dedicated crc+size cached wrapper")
		}
		GameMatchType::FileNameAndSize => {
			unreachable!("filename+size has a dedicated cached wrapper")
		}
		GameMatchType::NoMatch => {
			unreachable!("NoMatch is not a cacheable match type")
		}
	}
	.map(|(game, mappings)| IdentifyEntry {
		game,
		metadata_mappings: mappings,
	});

	let payload = serialize_option_redis_value(entry.clone())?;
	spawn_cache_write(
		redis_conn.clone(),
		cache_key,
		payload,
		IDENTIFY_CACHE_LIFETIME,
	);

	Ok(NonCached(entry))
}
