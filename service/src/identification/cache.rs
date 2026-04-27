use crate::cache::CacheStatus::{Cached, NonCached};
use crate::cache::{
	CACHE_KEY_VERSION, CACHE_PREFIX, CacheKey, CacheStatus, deserialize_option_redis_value,
	serialize_option_redis_value, spawn_cache_write,
};
use crate::db::game::{
	find_game_and_id_mapping_by_md5, find_game_and_id_mapping_by_name_and_size,
	find_game_and_id_mapping_by_sha1, find_game_and_id_mapping_by_sha256,
};
use crate::db::game_file::get_game_files_from_game_id;
use crate::error::ServiceResult;
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

#[derive(Debug, Clone, Copy)]
pub enum IdentifyCacheType {
	IdentifySha256,
	IdentifySha1,
	IdentifyMd5,
	IdentifyFilenameSize,
}

impl CacheKey for IdentifyCacheType {
	fn get_cache_key(&self, identifier: &str) -> String {
		match &self {
			IdentifyCacheType::IdentifySha256 => {
				format!("{CACHE_PREFIX}:cache:{CACHE_KEY_VERSION}:identify:sha256:{identifier}")
			}
			IdentifyCacheType::IdentifySha1 => {
				format!("{CACHE_PREFIX}:cache:{CACHE_KEY_VERSION}:identify:sha1:{identifier}")
			}
			IdentifyCacheType::IdentifyMd5 => {
				format!("{CACHE_PREFIX}:cache:{CACHE_KEY_VERSION}:identify:md5:{identifier}")
			}
			IdentifyCacheType::IdentifyFilenameSize => {
				format!(
					"{CACHE_PREFIX}:cache:{CACHE_KEY_VERSION}:identify:filename_size:{identifier}"
				)
			}
		}
	}
}

impl IdentifyCacheType {
	fn metric_label(&self) -> &'static str {
		match self {
			IdentifyCacheType::IdentifySha256 => "sha256",
			IdentifyCacheType::IdentifySha1 => "sha1",
			IdentifyCacheType::IdentifyMd5 => "md5",
			IdentifyCacheType::IdentifyFilenameSize => "filename_size",
		}
	}
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

pub async fn delete_identify_cache(
	hash: &str,
	r#type: IdentifyCacheType,
	redis_conn: &mut MultiplexedConnection,
) -> ServiceResult<()> {
	let cache_key = r#type.get_cache_key(hash);
	debug!("Deleting cache for key: {cache_key}");
	if let Err(e) = redis_conn.del(&cache_key).await {
		warn!("cache delete failed for {cache_key}: {e}");
	}
	Ok(())
}

/// Push every identify cache key derived from `file` (sha256, sha1, md5,
/// filename+size) into `out`. Skips fields that are missing on the model so
/// the caller can pipeline a single `DEL` over only the keys that exist.
pub fn collect_identify_cache_keys(file: &game_file::Model, out: &mut Vec<String>) {
	if let Some(sha256) = &file.sha256 {
		out.push(IdentifyCacheType::IdentifySha256.get_cache_key(sha256));
	}
	if let Some(sha1) = &file.sha1 {
		out.push(IdentifyCacheType::IdentifySha1.get_cache_key(sha1));
	}
	if let Some(md5) = &file.md5 {
		out.push(IdentifyCacheType::IdentifyMd5.get_cache_key(md5));
	}
	if let Some(size) = file.file_size_in_bytes {
		let key = filename_size_key(&file.file_name, size);
		out.push(IdentifyCacheType::IdentifyFilenameSize.get_cache_key(&key));
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
	let cache_key = IdentifyCacheType::IdentifyFilenameSize.get_cache_key(&key);

	if let Ok(Some(cached_val)) = redis_conn
		.get_ex(&cache_key, Expiry::EX(IDENTIFY_CACHE_LIFETIME))
		.await
	{
		debug!("Cache hit for filename+size");
		crate::metrics::record_cache_hit(
			"identify",
			IdentifyCacheType::IdentifyFilenameSize.metric_label(),
		);
		let deserialized = deserialize_option_redis_value(cached_val)?;
		return Ok(Cached(deserialized));
	}

	debug!("Cache miss for filename+size");
	crate::metrics::record_cache_miss(
		"identify",
		IdentifyCacheType::IdentifyFilenameSize.metric_label(),
	);

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
	r#type: IdentifyCacheType,
	redis_conn: &mut MultiplexedConnection,
	db_conn: &DbConn,
) -> ServiceResult<CacheStatus<Option<IdentifyEntry>>> {
	let cache_key = r#type.get_cache_key(hash);

	if let Ok(Some(cached_val)) = redis_conn
		.get_ex(&cache_key, Expiry::EX(IDENTIFY_CACHE_LIFETIME))
		.await
	{
		debug!("Cache hit for key: {hash}");
		crate::metrics::record_cache_hit("identify", r#type.metric_label());
		let deserialized = deserialize_option_redis_value(cached_val)?;
		return Ok(Cached(deserialized));
	}

	debug!("Cache miss for key: {hash}");
	crate::metrics::record_cache_miss("identify", r#type.metric_label());

	let entry = match r#type {
		IdentifyCacheType::IdentifySha256 => {
			find_game_and_id_mapping_by_sha256(hash, db_conn).await?
		}
		IdentifyCacheType::IdentifySha1 => find_game_and_id_mapping_by_sha1(hash, db_conn).await?,
		IdentifyCacheType::IdentifyMd5 => find_game_and_id_mapping_by_md5(hash, db_conn).await?,
		IdentifyCacheType::IdentifyFilenameSize => {
			unreachable!("filename+size has a dedicated cached wrapper")
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
