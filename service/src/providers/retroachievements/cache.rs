use crate::cache::{
	deserialize_option_redis_value, normalised_key_hash, provider_cache_key,
	serialize_option_redis_value, spawn_cache_write,
};
use crate::db::retroachievements::{
	find_retroachievements_game_by_id, find_retroachievements_game_by_md5,
	find_retroachievements_hashes_for_game, list_retroachievements_systems,
	search_retroachievements_games,
};
use crate::providers::retroachievements::model::{RaGame, RaGameHash, RaGameMatch, RaSystem};
use log::debug;
use redis::aio::MultiplexedConnection;
use redis::{AsyncTypedCommands, Expiry};
use sea_orm::DbConn;
use serde::Serialize;
use serde::de::DeserializeOwned;

const PROVIDER_LABEL: &str = "retroachievements";

const RA_CACHE_LIFETIME_GAME: u64 = 60 * 60 * 24;
const RA_CACHE_LIFETIME_SEARCH: u64 = 60 * 60 * 6;
const RA_CACHE_LIFETIME_CATALOG: u64 = 60 * 60 * 24 * 7;

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
		debug!("retroachievements cache hit for {label}: {cache_key}");
		crate::metrics::record_cache_hit(PROVIDER_LABEL, label);
		let deserialized = deserialize_option_redis_value(cached_val)?;
		return Ok(deserialized);
	}
	debug!("retroachievements cache miss for {label}: {cache_key}");
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
		debug!("retroachievements cache hit for {label}: {cache_key}");
		crate::metrics::record_cache_hit(PROVIDER_LABEL, label);
		let deserialized: Vec<T> = serde_json::from_str(&cached_val)?;
		return Ok(deserialized);
	}
	debug!("retroachievements cache miss for {label}: {cache_key}");
	crate::metrics::record_cache_miss(PROVIDER_LABEL, label);

	let values = fetch().await?;
	let payload = serde_json::to_string(&values)?;
	spawn_cache_write(redis_conn.clone(), cache_key, payload, ttl);
	Ok(values)
}

pub async fn get_ra_systems_cached(
	db_conn: &DbConn,
	redis_conn: &mut MultiplexedConnection,
) -> anyhow::Result<Vec<RaSystem>> {
	let cache_key = provider_cache_key(PROVIDER_LABEL, "systems", "all");
	cached_get_or_fetch_list(
		redis_conn,
		cache_key,
		RA_CACHE_LIFETIME_CATALOG,
		"Systems",
		|| async {
			let rows = list_retroachievements_systems(db_conn).await?;
			Ok(rows.into_iter().map(RaSystem::from).collect())
		},
	)
	.await
}

pub async fn get_ra_game_by_id_cached(
	db_conn: &DbConn,
	redis_conn: &mut MultiplexedConnection,
	game_id: i64,
) -> anyhow::Result<Option<RaGame>> {
	let cache_key = provider_cache_key(PROVIDER_LABEL, "game", &game_id.to_string());
	cached_get_or_fetch_single(
		redis_conn,
		cache_key,
		RA_CACHE_LIFETIME_GAME,
		"Game by Id",
		|| async {
			let row = find_retroachievements_game_by_id(game_id, db_conn).await?;
			Ok(row.map(RaGame::from))
		},
	)
	.await
}

pub async fn get_ra_game_by_md5_cached(
	db_conn: &DbConn,
	redis_conn: &mut MultiplexedConnection,
	md5: String,
) -> anyhow::Result<Option<RaGameMatch>> {
	let lower = md5.to_lowercase();
	let cache_key = provider_cache_key(PROVIDER_LABEL, "game:md5", &lower);
	cached_get_or_fetch_single(
		redis_conn,
		cache_key,
		RA_CACHE_LIFETIME_GAME,
		"Game by MD5",
		|| async {
			let Some(game) = find_retroachievements_game_by_md5(&lower, db_conn).await? else {
				return Ok(None);
			};
			let hashes = find_retroachievements_hashes_for_game(game.game_id, db_conn).await?;
			Ok(Some(RaGameMatch {
				game: RaGame::from(game),
				hashes: hashes.into_iter().map(RaGameHash::from).collect(),
			}))
		},
	)
	.await
}

pub async fn search_ra_games_cached(
	db_conn: &DbConn,
	redis_conn: &mut MultiplexedConnection,
	system_name: Option<String>,
	query: String,
) -> anyhow::Result<Vec<RaGame>> {
	let identifier = match &system_name {
		Some(s) => format!(
			"system:{}:{}",
			s.to_lowercase(),
			normalised_key_hash(&query)
		),
		None => normalised_key_hash(&query),
	};
	let cache_key = provider_cache_key(PROVIDER_LABEL, "game:search", &identifier);
	cached_get_or_fetch_list(
		redis_conn,
		cache_key,
		RA_CACHE_LIFETIME_SEARCH,
		"Game Search",
		|| async {
			let rows =
				search_retroachievements_games(system_name.as_deref(), &query, db_conn).await?;
			Ok(rows.into_iter().map(RaGame::from).collect())
		},
	)
	.await
}
