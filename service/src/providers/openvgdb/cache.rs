use crate::cache::{
	deserialize_option_redis_value, provider_cache_key, serialize_option_redis_value,
	spawn_cache_write,
};
use crate::db::openvgdb::{
	find_openvgdb_release_by_id, find_openvgdb_rom_by_crc, find_openvgdb_rom_by_md5,
	find_openvgdb_rom_by_sha1, list_openvgdb_releases_for_rom_id,
};
use crate::providers::openvgdb::model::{OvgdbRelease, OvgdbRom};
use log::debug;
use redis::aio::MultiplexedConnection;
use redis::{AsyncTypedCommands, Expiry};
use sea_orm::DbConn;
use serde::Serialize;
use serde::de::DeserializeOwned;

const PROVIDER_LABEL: &str = "openvgdb";

const OVGDB_CACHE_LIFETIME_RELEASE: u64 = 60 * 60 * 24;
const OVGDB_CACHE_LIFETIME_ROM: u64 = 60 * 60 * 24;

async fn cached_get_or_fetch_single<T, F, Fut>(
	redis_conn: &mut MultiplexedConnection,
	cache_key: String,
	ttl: u64,
	label: &'static str,
	fetch: F,
) -> anyhow::Result<Option<T>>
where
	T: Clone + Serialize + DeserializeOwned,
	F: FnOnce() -> Fut,
	Fut: std::future::Future<Output = anyhow::Result<Option<T>>>,
{
	if let Ok(Some(cached_val)) = redis_conn.get_ex(&cache_key, Expiry::EX(ttl)).await {
		debug!("OpenVGDB cache hit for {label}: {cache_key}");
		crate::metrics::record_cache_hit(PROVIDER_LABEL, label);
		let deserialized = deserialize_option_redis_value(cached_val)?;
		return Ok(deserialized);
	}
	debug!("OpenVGDB cache miss for {label}: {cache_key}");
	crate::metrics::record_cache_miss(PROVIDER_LABEL, label);

	let value = fetch().await?;
	let payload = serialize_option_redis_value(value.clone())?;
	spawn_cache_write(redis_conn.clone(), cache_key, payload, ttl);
	Ok(value)
}

async fn cached_get_or_fetch_list<T, F, Fut>(
	redis_conn: &mut MultiplexedConnection,
	cache_key: String,
	ttl: u64,
	label: &'static str,
	fetch: F,
) -> anyhow::Result<Vec<T>>
where
	T: Serialize + DeserializeOwned,
	F: FnOnce() -> Fut,
	Fut: std::future::Future<Output = anyhow::Result<Vec<T>>>,
{
	if let Ok(Some(cached_val)) = redis_conn.get_ex(&cache_key, Expiry::EX(ttl)).await {
		debug!("OpenVGDB cache hit for {label}: {cache_key}");
		crate::metrics::record_cache_hit(PROVIDER_LABEL, label);
		let deserialized: Vec<T> = serde_json::from_str(&cached_val)?;
		return Ok(deserialized);
	}
	debug!("OpenVGDB cache miss for {label}: {cache_key}");
	crate::metrics::record_cache_miss(PROVIDER_LABEL, label);

	let values = fetch().await?;
	let payload = serde_json::to_string(&values)?;
	spawn_cache_write(redis_conn.clone(), cache_key, payload, ttl);
	Ok(values)
}

pub async fn get_ovgdb_release_by_id_cached(
	db_conn: &DbConn,
	redis_conn: &mut MultiplexedConnection,
	release_id: i64,
) -> anyhow::Result<Option<OvgdbRelease>> {
	let cache_key = provider_cache_key(PROVIDER_LABEL, "release", &release_id.to_string());
	cached_get_or_fetch_single(
		redis_conn,
		cache_key,
		OVGDB_CACHE_LIFETIME_RELEASE,
		"Release by Id",
		|| async {
			let row = find_openvgdb_release_by_id(release_id, db_conn).await?;
			Ok(row.map(OvgdbRelease::from))
		},
	)
	.await
}

pub async fn get_ovgdb_releases_for_rom_cached(
	db_conn: &DbConn,
	redis_conn: &mut MultiplexedConnection,
	rom_id: i64,
) -> anyhow::Result<Vec<OvgdbRelease>> {
	let cache_key = provider_cache_key(PROVIDER_LABEL, "rom:releases", &rom_id.to_string());
	cached_get_or_fetch_list(
		redis_conn,
		cache_key,
		OVGDB_CACHE_LIFETIME_RELEASE,
		"Releases for Rom",
		|| async {
			let rows = list_openvgdb_releases_for_rom_id(rom_id, db_conn).await?;
			Ok(rows.into_iter().map(OvgdbRelease::from).collect())
		},
	)
	.await
}

pub async fn get_ovgdb_rom_by_hash_cached(
	db_conn: &DbConn,
	redis_conn: &mut MultiplexedConnection,
	kind: HashKind,
	hash: String,
) -> anyhow::Result<Option<OvgdbRom>> {
	let lower = hash.to_lowercase();
	let cache_key = provider_cache_key(
		PROVIDER_LABEL,
		"rom:hash",
		&format!("{}:{}", kind.label(), lower),
	);
	cached_get_or_fetch_single(
		redis_conn,
		cache_key,
		OVGDB_CACHE_LIFETIME_ROM,
		"Rom by Hash",
		|| async {
			let row = match kind {
				HashKind::Sha1 => find_openvgdb_rom_by_sha1(&lower, db_conn).await?,
				HashKind::Md5 => find_openvgdb_rom_by_md5(&lower, db_conn).await?,
				HashKind::Crc => find_openvgdb_rom_by_crc(&lower, db_conn).await?,
			};
			Ok(row.map(OvgdbRom::from))
		},
	)
	.await
}

#[derive(Debug, Clone, Copy)]
pub enum HashKind {
	Sha1,
	Md5,
	Crc,
}

impl HashKind {
	fn label(self) -> &'static str {
		match self {
			HashKind::Sha1 => "sha1",
			HashKind::Md5 => "md5",
			HashKind::Crc => "crc",
		}
	}
}
