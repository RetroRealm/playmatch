use crate::cache::{
	deserialize_option_redis_value, normalised_key_hash, provider_cache_key,
	serialize_option_redis_value, spawn_cache_write,
};
use crate::providers::mobygames::MobyGamesClient;
use crate::providers::mobygames::model::{
	MgCoversResp, MgGame, MgGenre, MgPlatform, MgScreenshotsResp,
};
use log::debug;
use redis::aio::MultiplexedConnection;
use redis::{AsyncTypedCommands, Expiry};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::Arc;

const PROVIDER_LABEL: &str = "mobygames";

const MG_CACHE_LIFETIME_CATALOG: u64 = 60 * 60 * 24 * 7;
const MG_CACHE_LIFETIME_GAME: u64 = 60 * 60 * 24;
const MG_CACHE_LIFETIME_SEARCH: u64 = 60 * 60 * 6;
const MG_CACHE_LIFETIME_ASSET: u64 = 60 * 60 * 6;

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
		debug!("mobygames cache hit for {label}: {cache_key}");
		crate::metrics::record_cache_hit(PROVIDER_LABEL, label);
		let deserialized = deserialize_option_redis_value(cached_val)?;
		return Ok(deserialized);
	}
	debug!("mobygames cache miss for {label}: {cache_key}");
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
		debug!("mobygames cache hit for {label}: {cache_key}");
		crate::metrics::record_cache_hit(PROVIDER_LABEL, label);
		let deserialized: Vec<T> = serde_json::from_str(&cached_val)?;
		return Ok(deserialized);
	}
	debug!("mobygames cache miss for {label}: {cache_key}");
	crate::metrics::record_cache_miss(PROVIDER_LABEL, label);

	let values = fetch().await?;
	let payload = serde_json::to_string(&values)?;
	spawn_cache_write(redis_conn.clone(), cache_key, payload, ttl);
	Ok(values)
}

async fn cached_get_or_fetch_value<T, F, Fut>(
	redis_conn: &mut MultiplexedConnection,
	cache_key: String,
	ttl: u64,
	label: &'static str,
	fetch: F,
) -> anyhow::Result<T>
where
	T: Clone + Serialize + DeserializeOwned,
	F: FnOnce() -> Fut,
	Fut: std::future::Future<Output = anyhow::Result<T>>,
{
	if let Ok(Some(cached_val)) = redis_conn.get_ex(&cache_key, Expiry::EX(ttl)).await {
		debug!("mobygames cache hit for {label}: {cache_key}");
		crate::metrics::record_cache_hit(PROVIDER_LABEL, label);
		let deserialized: T = serde_json::from_str(&cached_val)?;
		return Ok(deserialized);
	}
	debug!("mobygames cache miss for {label}: {cache_key}");
	crate::metrics::record_cache_miss(PROVIDER_LABEL, label);

	let value = fetch().await?;
	let payload = serde_json::to_string(&value)?;
	spawn_cache_write(redis_conn.clone(), cache_key, payload, ttl);
	Ok(value)
}

pub async fn get_mg_platforms_cached(
	client: &MobyGamesClient,
	redis_conn: &mut MultiplexedConnection,
) -> anyhow::Result<Vec<MgPlatform>> {
	let cache_key = provider_cache_key(PROVIDER_LABEL, "platforms", "all");
	cached_get_or_fetch_list(
		redis_conn,
		cache_key,
		MG_CACHE_LIFETIME_CATALOG,
		"Platforms",
		|| async {
			let arc: Arc<Vec<MgPlatform>> = client.list_platforms().await?;
			Ok((*arc).clone())
		},
	)
	.await
}

pub async fn get_mg_genres_cached(
	client: &MobyGamesClient,
	redis_conn: &mut MultiplexedConnection,
) -> anyhow::Result<Vec<MgGenre>> {
	let cache_key = provider_cache_key(PROVIDER_LABEL, "genres", "all");
	cached_get_or_fetch_list(
		redis_conn,
		cache_key,
		MG_CACHE_LIFETIME_CATALOG,
		"Genres",
		|| async {
			let arc: Arc<Vec<MgGenre>> = client.list_genres().await?;
			Ok((*arc).clone())
		},
	)
	.await
}

pub async fn search_mg_games_cached(
	client: &MobyGamesClient,
	redis_conn: &mut MultiplexedConnection,
	platform_id: Option<i64>,
	query: String,
) -> anyhow::Result<Vec<MgGame>> {
	let identifier = match platform_id {
		Some(pid) => format!("platform:{pid}:{}", normalised_key_hash(&query)),
		None => normalised_key_hash(&query),
	};
	let cache_key = provider_cache_key(PROVIDER_LABEL, "game:search", &identifier);
	cached_get_or_fetch_list(
		redis_conn,
		cache_key,
		MG_CACHE_LIFETIME_SEARCH,
		"Game Search",
		|| async { client.search_games(platform_id, &query).await },
	)
	.await
}

pub async fn get_mg_game_by_id_cached(
	client: &MobyGamesClient,
	redis_conn: &mut MultiplexedConnection,
	id: i64,
) -> anyhow::Result<Option<MgGame>> {
	let cache_key = provider_cache_key(PROVIDER_LABEL, "game", &id.to_string());
	cached_get_or_fetch_single(
		redis_conn,
		cache_key,
		MG_CACHE_LIFETIME_GAME,
		"Game by Id",
		|| async { client.get_game_by_id(id).await },
	)
	.await
}

pub async fn get_mg_game_platform_covers_cached(
	client: &MobyGamesClient,
	redis_conn: &mut MultiplexedConnection,
	game_id: i64,
	platform_id: i64,
) -> anyhow::Result<MgCoversResp> {
	let cache_key = provider_cache_key(
		PROVIDER_LABEL,
		"game:covers",
		&format!("{game_id}:{platform_id}"),
	);
	cached_get_or_fetch_value(
		redis_conn,
		cache_key,
		MG_CACHE_LIFETIME_ASSET,
		"Covers",
		|| async { client.get_game_platform_covers(game_id, platform_id).await },
	)
	.await
}

pub async fn get_mg_game_platform_screenshots_cached(
	client: &MobyGamesClient,
	redis_conn: &mut MultiplexedConnection,
	game_id: i64,
	platform_id: i64,
) -> anyhow::Result<MgScreenshotsResp> {
	let cache_key = provider_cache_key(
		PROVIDER_LABEL,
		"game:screenshots",
		&format!("{game_id}:{platform_id}"),
	);
	cached_get_or_fetch_value(
		redis_conn,
		cache_key,
		MG_CACHE_LIFETIME_ASSET,
		"Screenshots",
		|| async {
			client
				.get_game_platform_screenshots(game_id, platform_id)
				.await
		},
	)
	.await
}
