use crate::cache::{
	deserialize_option_redis_value, normalised_key_hash, provider_cache_key,
	serialize_option_redis_value, spawn_cache_write,
};
use crate::db::launchbox::{
	find_lb_game_alternate_names, find_lb_game_by_database_id, find_lb_game_images,
	list_lb_platforms, search_lb_games,
};
use crate::providers::launchbox::model::{LbGame, LbGameAlternateName, LbGameImage, LbPlatform};
use log::debug;
use redis::aio::MultiplexedConnection;
use redis::{AsyncTypedCommands, Expiry};
use sea_orm::DbConn;
use serde::Serialize;
use serde::de::DeserializeOwned;

const PROVIDER_LABEL: &str = "launchbox";

const LB_CACHE_LIFETIME_GAME: u64 = 60 * 60 * 24;
const LB_CACHE_LIFETIME_SEARCH: u64 = 60 * 60 * 6;
const LB_CACHE_LIFETIME_CATALOG: u64 = 60 * 60 * 24 * 7;

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
		debug!("LaunchBox cache hit for {label}: {cache_key}");
		crate::metrics::record_cache_hit(PROVIDER_LABEL, label);
		let deserialized = deserialize_option_redis_value(cached_val)?;
		return Ok(deserialized);
	}
	debug!("LaunchBox cache miss for {label}: {cache_key}");
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
		debug!("LaunchBox cache hit for {label}: {cache_key}");
		crate::metrics::record_cache_hit(PROVIDER_LABEL, label);
		let deserialized: Vec<T> = serde_json::from_str(&cached_val)?;
		return Ok(deserialized);
	}
	debug!("LaunchBox cache miss for {label}: {cache_key}");
	crate::metrics::record_cache_miss(PROVIDER_LABEL, label);

	let values = fetch().await?;
	let payload = serde_json::to_string(&values)?;
	spawn_cache_write(redis_conn.clone(), cache_key, payload, ttl);
	Ok(values)
}

pub async fn get_lb_platforms_cached(
	db_conn: &DbConn,
	redis_conn: &mut MultiplexedConnection,
) -> anyhow::Result<Vec<LbPlatform>> {
	let cache_key = provider_cache_key(PROVIDER_LABEL, "platforms", "all");
	cached_get_or_fetch_list(
		redis_conn,
		cache_key,
		LB_CACHE_LIFETIME_CATALOG,
		"Platforms",
		|| async {
			let rows = list_lb_platforms(db_conn).await?;
			Ok(rows.into_iter().map(LbPlatform::from).collect())
		},
	)
	.await
}

pub async fn get_lb_game_by_id_cached(
	db_conn: &DbConn,
	redis_conn: &mut MultiplexedConnection,
	id: i64,
) -> anyhow::Result<Option<LbGame>> {
	let cache_key = provider_cache_key(PROVIDER_LABEL, "game", &id.to_string());
	cached_get_or_fetch_single(
		redis_conn,
		cache_key,
		LB_CACHE_LIFETIME_GAME,
		"Game by Id",
		|| async {
			let row = find_lb_game_by_database_id(id, db_conn).await?;
			Ok(row.map(LbGame::from))
		},
	)
	.await
}

pub async fn search_lb_games_cached(
	db_conn: &DbConn,
	redis_conn: &mut MultiplexedConnection,
	platform_name: Option<String>,
	query: String,
) -> anyhow::Result<Vec<LbGame>> {
	let identifier = match &platform_name {
		Some(p) => format!(
			"platform:{}:{}",
			p.to_lowercase(),
			normalised_key_hash(&query)
		),
		None => normalised_key_hash(&query),
	};
	let cache_key = provider_cache_key(PROVIDER_LABEL, "game:search", &identifier);
	cached_get_or_fetch_list(
		redis_conn,
		cache_key,
		LB_CACHE_LIFETIME_SEARCH,
		"Game Search",
		|| async {
			let rows = search_lb_games(platform_name.as_deref(), &query, db_conn).await?;
			Ok(rows.into_iter().map(LbGame::from).collect())
		},
	)
	.await
}

pub async fn get_lb_game_alternate_names_cached(
	db_conn: &DbConn,
	redis_conn: &mut MultiplexedConnection,
	game_id: i64,
) -> anyhow::Result<Vec<LbGameAlternateName>> {
	let cache_key =
		provider_cache_key(PROVIDER_LABEL, "game:alternate-names", &game_id.to_string());
	cached_get_or_fetch_list(
		redis_conn,
		cache_key,
		LB_CACHE_LIFETIME_GAME,
		"Game Alternate Names",
		|| async {
			let rows = find_lb_game_alternate_names(game_id, db_conn).await?;
			Ok(rows.into_iter().map(LbGameAlternateName::from).collect())
		},
	)
	.await
}

pub async fn get_lb_game_images_cached(
	db_conn: &DbConn,
	redis_conn: &mut MultiplexedConnection,
	game_id: i64,
) -> anyhow::Result<Vec<LbGameImage>> {
	let cache_key = provider_cache_key(PROVIDER_LABEL, "game:images", &game_id.to_string());
	cached_get_or_fetch_list(
		redis_conn,
		cache_key,
		LB_CACHE_LIFETIME_GAME,
		"Game Images",
		|| async {
			let rows = find_lb_game_images(game_id, db_conn).await?;
			Ok(rows.into_iter().map(LbGameImage::from).collect())
		},
	)
	.await
}
