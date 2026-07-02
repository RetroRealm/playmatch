use crate::cache::{
	deserialize_option_redis_value, normalised_key_hash, provider_cache_key,
	serialize_option_redis_value, spawn_cache_write,
};
use crate::providers::steamgriddb::SteamGridDbClient;
use crate::providers::steamgriddb::model::{AssetFilters, SgdbAsset, SgdbGame, SgdbPlatform};
use hex::encode as hex_encode;
use log::debug;
use redis::aio::MultiplexedConnection;
use redis::{AsyncTypedCommands, Expiry};
use serde::Serialize;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};

const PROVIDER_LABEL: &str = "steamgriddb";

const SGDB_CACHE_LIFETIME_GAME: u64 = 60 * 60 * 24;
const SGDB_CACHE_LIFETIME_ASSET: u64 = 60 * 60 * 6;
const SGDB_CACHE_LIFETIME_SEARCH: u64 = 60 * 60 * 6;

/// Hash an `AssetFilters` canonical string into a short hex digest. Used as
/// the trailing component of the cache key so different filter combinations
/// share the segment but not the entry.
fn filters_hash(filters: &AssetFilters) -> String {
	hex_encode(Sha256::digest(filters.canonical_string().as_bytes()))
}

fn external_key(platform: SgdbPlatform, platform_id: &str) -> String {
	format!("{}:{platform_id}", platform.as_str())
}

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
		debug!("SteamGridDB cache hit for {label}: {cache_key}");
		crate::metrics::record_cache_hit(PROVIDER_LABEL, label);
		let deserialized = deserialize_option_redis_value(cached_val)?;
		return Ok(deserialized);
	}
	debug!("SteamGridDB cache miss for {label}: {cache_key}");
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
		debug!("SteamGridDB cache hit for {label}: {cache_key}");
		crate::metrics::record_cache_hit(PROVIDER_LABEL, label);
		let deserialized: Vec<T> = serde_json::from_str(&cached_val)?;
		return Ok(deserialized);
	}
	debug!("SteamGridDB cache miss for {label}: {cache_key}");
	crate::metrics::record_cache_miss(PROVIDER_LABEL, label);

	let values = fetch().await?;
	let payload = serde_json::to_string(&values)?;
	spawn_cache_write(redis_conn.clone(), cache_key, payload, ttl);
	Ok(values)
}

pub async fn get_sgdb_game_by_id_cached(
	client: &SteamGridDbClient,
	redis_conn: &mut MultiplexedConnection,
	id: i64,
) -> anyhow::Result<Option<SgdbGame>> {
	let cache_key = provider_cache_key(PROVIDER_LABEL, "game", &id.to_string());
	cached_get_or_fetch_single(
		redis_conn,
		cache_key,
		SGDB_CACHE_LIFETIME_GAME,
		"Game by Id",
		|| async { client.get_game_by_id(id).await },
	)
	.await
}

pub async fn get_sgdb_game_by_platform_cached(
	client: &SteamGridDbClient,
	redis_conn: &mut MultiplexedConnection,
	platform: SgdbPlatform,
	platform_id: String,
) -> anyhow::Result<Option<SgdbGame>> {
	let segment = "game:platform";
	let cache_key = provider_cache_key(
		PROVIDER_LABEL,
		segment,
		&external_key(platform, &platform_id),
	);
	cached_get_or_fetch_single(
		redis_conn,
		cache_key,
		SGDB_CACHE_LIFETIME_GAME,
		"Game by Platform",
		|| async { client.get_game_by_platform(platform, &platform_id).await },
	)
	.await
}

pub async fn search_sgdb_games_cached(
	client: &SteamGridDbClient,
	redis_conn: &mut MultiplexedConnection,
	query: String,
) -> anyhow::Result<Vec<SgdbGame>> {
	let cache_key = provider_cache_key(PROVIDER_LABEL, "game:search", &normalised_key_hash(&query));
	cached_get_or_fetch_list(
		redis_conn,
		cache_key,
		SGDB_CACHE_LIFETIME_SEARCH,
		"Game Search",
		|| async { client.search_games(&query).await },
	)
	.await
}

macro_rules! sgdb_assets_by_game {
	($fn_name:ident, $segment:literal, $client_method:ident, $label:literal) => {
		pub async fn $fn_name(
			client: &SteamGridDbClient,
			redis_conn: &mut MultiplexedConnection,
			game_id: i64,
			filters: AssetFilters,
		) -> anyhow::Result<Vec<SgdbAsset>> {
			let cache_key = provider_cache_key(
				PROVIDER_LABEL,
				$segment,
				&format!("{game_id}:{}", filters_hash(&filters)),
			);
			cached_get_or_fetch_list(
				redis_conn,
				cache_key,
				SGDB_CACHE_LIFETIME_ASSET,
				$label,
				|| async { client.$client_method(game_id, &filters).await },
			)
			.await
		}
	};
}

macro_rules! sgdb_assets_by_platform {
	($fn_name:ident, $segment:literal, $client_method:ident, $label:literal) => {
		pub async fn $fn_name(
			client: &SteamGridDbClient,
			redis_conn: &mut MultiplexedConnection,
			platform: SgdbPlatform,
			platform_id: String,
			filters: AssetFilters,
		) -> anyhow::Result<Vec<SgdbAsset>> {
			let cache_key = provider_cache_key(
				PROVIDER_LABEL,
				$segment,
				&format!(
					"{}:{}",
					external_key(platform, &platform_id),
					filters_hash(&filters)
				),
			);
			cached_get_or_fetch_list(
				redis_conn,
				cache_key,
				SGDB_CACHE_LIFETIME_ASSET,
				$label,
				|| async {
					client
						.$client_method(platform, &platform_id, &filters)
						.await
				},
			)
			.await
		}
	};
}

sgdb_assets_by_game!(
	get_sgdb_grids_by_game_cached,
	"grid:game",
	get_grids_by_game,
	"Grids by Game"
);
sgdb_assets_by_platform!(
	get_sgdb_grids_by_platform_cached,
	"grid:platform",
	get_grids_by_platform,
	"Grids by Platform"
);

sgdb_assets_by_game!(
	get_sgdb_heroes_by_game_cached,
	"hero:game",
	get_heroes_by_game,
	"Heroes by Game"
);
sgdb_assets_by_platform!(
	get_sgdb_heroes_by_platform_cached,
	"hero:platform",
	get_heroes_by_platform,
	"Heroes by Platform"
);

sgdb_assets_by_game!(
	get_sgdb_logos_by_game_cached,
	"logo:game",
	get_logos_by_game,
	"Logos by Game"
);
sgdb_assets_by_platform!(
	get_sgdb_logos_by_platform_cached,
	"logo:platform",
	get_logos_by_platform,
	"Logos by Platform"
);

sgdb_assets_by_game!(
	get_sgdb_icons_by_game_cached,
	"icon:game",
	get_icons_by_game,
	"Icons by Game"
);
sgdb_assets_by_platform!(
	get_sgdb_icons_by_platform_cached,
	"icon:platform",
	get_icons_by_platform,
	"Icons by Platform"
);
