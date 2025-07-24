use crate::error;
use actix_web::web::Data;
use actix_web::{HttpResponse, Responder, get, web};
use log::debug;
use sea_orm::DatabaseConnection;
use service::cache::CacheStatus;
use service::game::{identify_game_and_get_relations, identify_game_and_metadata_mappings};
use service::model::GameFileMatchSearch;
use web::Query;

/// Identify a game by its file hashes or filename and size, returning the matched metadata, goes in order sha256, sha1, md5 and filename + size (from most accurate to least accurate)
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "Identify",
	params(GameFileMatchSearch),
	responses(
		(status = 200, description = "Returns info about a possible match via hashes or filename and size", body = GameMetadataMatchResult)
	)
)]
#[get("/identify/ids")]
pub async fn identify_game_with_metadata_ids(
	query: Query<GameFileMatchSearch>,
	db_conn: Data<DatabaseConnection>,
	redis_client: Data<redis::Client>,
) -> error::Result<impl Responder> {
	let identify_result = identify_game_and_metadata_mappings(
		query.into_inner(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		db_conn.get_ref(),
	)
	.await?;

	let cache_status = match identify_result {
		CacheStatus::Cached(match_result) => ("HIT", match_result),
		CacheStatus::NonCached(match_result) => ("MISS", match_result),
	};

	debug!(
		"Cache {} for identify game and get metadata ids",
		cache_status.0
	);

	Ok(HttpResponse::Ok()
		.append_header(("X-Cache", cache_status.0))
		.json(cache_status.1))
}

/// Identify a game by its file hashes or filename and size, goes in order sha256, sha1, md5 and filename + size (from most accurate to least accurate), returning information about the game, game files, metadata mappings, publisher and company
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "Identify",
	params(GameFileMatchSearch),
	responses(
		(status = 200, description = "Returns info about a possible match via hashes or filename and size", body = GameAndRelationMatchResult)
	)
)]
#[get("/identify/relations")]
pub async fn identify_game_and_relations(
	query: Query<GameFileMatchSearch>,
	db_conn: Data<DatabaseConnection>,
	redis_client: Data<redis::Client>,
) -> error::Result<impl Responder> {
	let identify_result = identify_game_and_get_relations(
		query.into_inner(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		db_conn.get_ref(),
	)
	.await?;

	let cache_status = match identify_result {
		CacheStatus::Cached(match_result) => ("HIT", match_result),
		CacheStatus::NonCached(match_result) => ("MISS", match_result),
	};

	debug!("Cache {} for identify game and relations", cache_status.0);

	Ok(HttpResponse::Ok()
		.append_header(("X-Cache", cache_status.0))
		.json(cache_status.1))
}
