use crate::cache::CacheStatus::{Cached, NonCached};
use crate::cache::{
	CACHE_PREFIX, CacheKey, CacheStatus, deserialize_option_redis_value,
	serialize_option_redis_value,
};
use crate::db::game::{
	find_game_and_id_mapping_by_md5, find_game_and_id_mapping_by_sha1,
	find_game_and_id_mapping_by_sha256,
};
use crate::error::ServiceResult;
use entity::{game, signature_metadata_mapping};
use log::{debug, warn};
use redis::AsyncTypedCommands;
use redis::aio::MultiplexedConnection;
use sea_orm::DbConn;
use serde::{Deserialize, Serialize};
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
}

impl CacheKey for IdentifyCacheType {
	fn get_cache_key(&self, identifier: &str) -> String {
		match &self {
			IdentifyCacheType::IdentifySha256 => {
				format!("{CACHE_PREFIX}:cache:identify:sha256:{identifier}")
			}
			IdentifyCacheType::IdentifySha1 => {
				format!("{CACHE_PREFIX}:cache:identify:sha1:{identifier}")
			}
			IdentifyCacheType::IdentifyMd5 => {
				format!("{CACHE_PREFIX}:cache:identify:md5:{identifier}")
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
		}
	}
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

pub async fn find_game_and_metadata_ids_by_sha256_cached(
	sha256: &str,
	redis_conn: &mut MultiplexedConnection,
	db_conn: &DbConn,
) -> ServiceResult<CacheStatus<Option<IdentifyEntry>>> {
	find_game_and_metadata_ids_cached(
		sha256,
		IdentifyCacheType::IdentifySha256,
		redis_conn,
		db_conn,
	)
	.await
}

pub async fn find_game_and_metadata_ids_by_sha1_cached(
	sha1: &str,
	redis_conn: &mut MultiplexedConnection,
	db_conn: &DbConn,
) -> ServiceResult<CacheStatus<Option<IdentifyEntry>>> {
	find_game_and_metadata_ids_cached(sha1, IdentifyCacheType::IdentifySha1, redis_conn, db_conn)
		.await
}

pub async fn find_game_and_metadata_ids_by_md5_cached(
	md5: &str,
	redis_conn: &mut MultiplexedConnection,
	db_conn: &DbConn,
) -> ServiceResult<CacheStatus<Option<IdentifyEntry>>> {
	find_game_and_metadata_ids_cached(md5, IdentifyCacheType::IdentifyMd5, redis_conn, db_conn)
		.await
}

async fn find_game_and_metadata_ids_cached(
	hash: &str,
	r#type: IdentifyCacheType,
	redis_conn: &mut MultiplexedConnection,
	db_conn: &DbConn,
) -> ServiceResult<CacheStatus<Option<IdentifyEntry>>> {
	let cache_key = r#type.get_cache_key(hash);

	if let Ok(Some(cached_val)) = redis_conn.get(&cache_key).await {
		debug!("Cache hit for key: {hash}");
		crate::metrics::record_cache_hit("identify", r#type.metric_label());
		if let Err(e) = redis_conn
			.expire(&cache_key, IDENTIFY_CACHE_LIFETIME as i64)
			.await
		{
			warn!("cache ttl refresh failed for {cache_key}: {e}");
		}
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
	}
	.map(|(game, mappings)| IdentifyEntry {
		game,
		metadata_mappings: mappings,
	});

	if let Err(e) = redis_conn
		.set_ex(
			&cache_key,
			serialize_option_redis_value(entry.clone())?,
			IDENTIFY_CACHE_LIFETIME,
		)
		.await
	{
		warn!("cache write failed for {cache_key}: {e}");
	}

	Ok(NonCached(entry))
}
