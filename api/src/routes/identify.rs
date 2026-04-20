use crate::error;
use actix_web::web::Data;
use actix_web::{HttpResponse, Responder, get, web};
use log::debug;
use redis::aio::MultiplexedConnection;
use sea_orm::DatabaseConnection;
use serde::Serialize;
use service::cache::CacheStatus;
use service::identification::{
	identify_game_and_get_relations, identify_game_and_metadata_mappings,
};
use service::model::GameFileMatchSearch;
use web::Query;

fn cache_status_response<T: Serialize>(status: CacheStatus<T>, log_label: &str) -> HttpResponse {
	let (tag, body) = match status {
		CacheStatus::Cached(v) => ("HIT", v),
		CacheStatus::NonCached(v) => ("MISS", v),
	};
	debug!("Cache {tag} for {log_label}");
	HttpResponse::Ok()
		.append_header(("X-Cache", tag))
		.json(body)
}

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
	redis_conn: Data<MultiplexedConnection>,
) -> error::Result<impl Responder> {
	let query = query.into_inner();
	if let Err(msg) = query.validate() {
		return Ok(HttpResponse::BadRequest().body(msg));
	}

	let mut redis_conn = redis_conn.get_ref().clone();
	let identify_result =
		identify_game_and_metadata_mappings(query, &mut redis_conn, db_conn.get_ref()).await?;

	Ok(cache_status_response(
		identify_result,
		"identify game and get metadata ids",
	))
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
	redis_conn: Data<MultiplexedConnection>,
) -> error::Result<impl Responder> {
	let query = query.into_inner();
	if let Err(msg) = query.validate() {
		return Ok(HttpResponse::BadRequest().body(msg));
	}

	let mut redis_conn = redis_conn.get_ref().clone();
	let identify_result =
		identify_game_and_get_relations(query, &mut redis_conn, db_conn.get_ref()).await?;

	Ok(cache_status_response(
		identify_result,
		"identify game and relations",
	))
}
