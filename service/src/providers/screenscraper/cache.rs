use crate::cache::{
	deserialize_option_redis_value, normalised_key_hash, provider_cache_key,
	serialize_option_redis_value, spawn_cache_write,
};
use crate::providers::screenscraper::ScreenScraperClient;
use crate::providers::screenscraper::model::{SsGame, SsSystem};
use log::debug;
use redis::aio::MultiplexedConnection;
use redis::{AsyncTypedCommands, Expiry};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::Arc;

const PROVIDER_LABEL: &str = "screenscraper";

const SS_CACHE_LIFETIME_SYSTEMS: u64 = 60 * 60 * 24 * 7;
const SS_CACHE_LIFETIME_GAME: u64 = 60 * 60 * 24;
const SS_CACHE_LIFETIME_SEARCH: u64 = 60 * 60 * 6;

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
		debug!("screenscraper cache hit for {label}: {cache_key}");
		crate::metrics::record_cache_hit(PROVIDER_LABEL, label);
		let deserialized = deserialize_option_redis_value(cached_val)?;
		return Ok(deserialized);
	}
	debug!("screenscraper cache miss for {label}: {cache_key}");
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
		debug!("screenscraper cache hit for {label}: {cache_key}");
		crate::metrics::record_cache_hit(PROVIDER_LABEL, label);
		let deserialized: Vec<T> = serde_json::from_str(&cached_val)?;
		return Ok(deserialized);
	}
	debug!("screenscraper cache miss for {label}: {cache_key}");
	crate::metrics::record_cache_miss(PROVIDER_LABEL, label);

	let values = fetch().await?;
	let payload = serde_json::to_string(&values)?;
	spawn_cache_write(redis_conn.clone(), cache_key, payload, ttl);
	Ok(values)
}

pub async fn get_ss_systems_cached(
	client: &ScreenScraperClient,
	redis_conn: &mut MultiplexedConnection,
) -> anyhow::Result<Vec<SsSystem>> {
	let cache_key = provider_cache_key(PROVIDER_LABEL, "systems", "all");
	cached_get_or_fetch_list(
		redis_conn,
		cache_key,
		SS_CACHE_LIFETIME_SYSTEMS,
		"Systems",
		|| async {
			let arc: Arc<Vec<SsSystem>> = client.list_systems().await?;
			Ok((*arc).clone())
		},
	)
	.await
}

pub async fn get_ss_game_by_id_cached(
	client: &ScreenScraperClient,
	redis_conn: &mut MultiplexedConnection,
	id: i64,
) -> anyhow::Result<Option<SsGame>> {
	let cache_key = provider_cache_key(PROVIDER_LABEL, "game", &id.to_string());
	cached_get_or_fetch_single(
		redis_conn,
		cache_key,
		SS_CACHE_LIFETIME_GAME,
		"Game by Id",
		|| async { client.get_game_by_id(id).await },
	)
	.await
}

pub async fn get_ss_game_by_rom_name_cached(
	client: &ScreenScraperClient,
	redis_conn: &mut MultiplexedConnection,
	system_id: i32,
	rom_name: String,
) -> anyhow::Result<Option<SsGame>> {
	let cache_key = provider_cache_key(
		PROVIDER_LABEL,
		"game:rom",
		&format!("{system_id}:{}", normalised_key_hash(&rom_name)),
	);
	cached_get_or_fetch_single(
		redis_conn,
		cache_key,
		SS_CACHE_LIFETIME_SEARCH,
		"Game by Rom",
		|| async { client.get_game_by_rom_name(system_id, &rom_name).await },
	)
	.await
}

pub async fn search_ss_games_cached(
	client: &ScreenScraperClient,
	redis_conn: &mut MultiplexedConnection,
	system_id: i32,
	query: String,
) -> anyhow::Result<Vec<SsGame>> {
	let cache_key = provider_cache_key(
		PROVIDER_LABEL,
		"game:search",
		&format!("{system_id}:{}", normalised_key_hash(&query)),
	);
	cached_get_or_fetch_list(
		redis_conn,
		cache_key,
		SS_CACHE_LIFETIME_SEARCH,
		"Game Search",
		|| async { client.search_games(system_id, &query).await },
	)
	.await
}

pub async fn get_ss_game_by_md5_cached(
	client: &ScreenScraperClient,
	redis_conn: &mut MultiplexedConnection,
	system_id: i32,
	rom_name: String,
	rom_size: Option<i64>,
	md5: String,
) -> anyhow::Result<Option<SsGame>> {
	let lower = md5.to_lowercase();
	let cache_key = provider_cache_key(PROVIDER_LABEL, "game:md5", &format!("{system_id}:{lower}"));
	cached_get_or_fetch_single(
		redis_conn,
		cache_key,
		SS_CACHE_LIFETIME_GAME,
		"Game by MD5",
		|| async {
			client
				.get_game_by_md5(system_id, &rom_name, rom_size, &lower)
				.await
		},
	)
	.await
}

pub async fn get_ss_game_by_sha1_cached(
	client: &ScreenScraperClient,
	redis_conn: &mut MultiplexedConnection,
	system_id: i32,
	rom_name: String,
	rom_size: Option<i64>,
	sha1: String,
) -> anyhow::Result<Option<SsGame>> {
	let lower = sha1.to_lowercase();
	let cache_key =
		provider_cache_key(PROVIDER_LABEL, "game:sha1", &format!("{system_id}:{lower}"));
	cached_get_or_fetch_single(
		redis_conn,
		cache_key,
		SS_CACHE_LIFETIME_GAME,
		"Game by SHA1",
		|| async {
			client
				.get_game_by_sha1(system_id, &rom_name, rom_size, &lower)
				.await
		},
	)
	.await
}

pub async fn get_ss_game_by_crc_cached(
	client: &ScreenScraperClient,
	redis_conn: &mut MultiplexedConnection,
	system_id: i32,
	rom_name: String,
	rom_size: Option<i64>,
	crc: String,
) -> anyhow::Result<Option<SsGame>> {
	let lower = crc.to_lowercase();
	let cache_key = provider_cache_key(PROVIDER_LABEL, "game:crc", &format!("{system_id}:{lower}"));
	cached_get_or_fetch_single(
		redis_conn,
		cache_key,
		SS_CACHE_LIFETIME_GAME,
		"Game by CRC",
		|| async {
			client
				.get_game_by_crc(system_id, &rom_name, rom_size, &lower)
				.await
		},
	)
	.await
}
