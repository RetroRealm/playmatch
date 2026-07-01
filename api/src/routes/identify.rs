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

/// Identifies a game by its file hashes or by filename and size.
///
/// The strongest supplied hash resolves the file: sha256, then sha1, then md5,
/// then crc. When no hash matches, the filename and size are tried last. The
/// result carries the matched game along with its metadata provider ids.
#[utoipa::path(
	get,
	tag = "Identify",
	params(GameFileMatchSearch),
	responses(
		(status = 200, description = "The match result, including a no-match outcome", body = GameMetadataMatchResult)
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

/// Identifies a game by its file hashes or by filename and size.
///
/// The strongest supplied hash resolves the file: sha256, then sha1, then md5,
/// then crc. When no hash matches, the filename and size are tried last. The
/// result carries the matched game together with its game files, metadata
/// mappings, publisher, and company.
#[utoipa::path(
	get,
	tag = "Identify",
	params(GameFileMatchSearch),
	responses(
		(status = 200, description = "The match result and its related records, including a no-match outcome", body = GameAndRelationMatchResult)
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
