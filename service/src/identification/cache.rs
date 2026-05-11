use crate::cache::CacheStatus::{Cached, NonCached};
use crate::cache::{
	CACHE_KEY_VERSION, CACHE_PREFIX, CacheStatus, deserialize_option_redis_value,
	serialize_option_redis_value, spawn_cache_write,
};
use crate::db::game::{
	find_game_and_id_mapping_by_md5, find_game_and_id_mapping_by_name_and_size,
	find_game_and_id_mapping_by_sha1, find_game_and_id_mapping_by_sha256,
};
use crate::db::game_file::get_game_files_from_game_id;
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
use std::time::Duration;

const IDENTIFY_CACHE_LIFETIME: u64 = Duration::from_secs(60 * 60 * 24 * 7).as_secs(); // 7 days

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IdentifyEntry {
	pub game: game::Model,
	pub metadata_mappings: Vec<signature_metadata_mapping::Model>,
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
