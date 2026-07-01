use crate::error;
use crate::routes::v2::error::v2_bad_request;
use actix_web::web::{Data, Query};
use actix_web::{HttpResponse, Responder, get};
use log::debug;
use redis::aio::MultiplexedConnection;
use sea_orm::DatabaseConnection;
use service::cache::CacheStatus;
use service::identification::{
	identify_game_and_get_relations_v2, identify_game_and_metadata_mappings,
};
use service::model::GameFileMatchSearch;

/// Identifies a game by its file hashes or by filename and size, returning the
/// matched game and its metadata provider ids.
///
/// The strongest supplied hash resolves the file: sha256, then sha1, then md5,
/// then crc. When no hash matches, the filename and size are tried last.
///
/// This is the v2 twin of the v1 `/identify/ids`: identical behaviour, except a
/// malformed query answers the v2 `{code, message}` envelope instead of v1's
/// plain-text body.
#[utoipa::path(
	get,
	tag = "Identify",
	params(GameFileMatchSearch),
	responses(
		(status = 200, description = "The match result, including a no-match outcome", body = GameMetadataMatchResult),
		(status = 400, description = "Invalid query parameters", body = V2ErrorBody)
	)
)]
#[get("/identify/ids")]
pub async fn identify_game_with_metadata_ids_v2(
	query: Query<GameFileMatchSearch>,
	db_conn: Data<DatabaseConnection>,
	redis_conn: Data<MultiplexedConnection>,
) -> error::Result<impl Responder> {
	let query = query.into_inner();
	if let Err(msg) = query.validate() {
		return Ok(v2_bad_request("invalid_query", msg));
	}

	let mut redis_conn = redis_conn.get_ref().clone();
	let identify_result =
		identify_game_and_metadata_mappings(query, &mut redis_conn, db_conn.get_ref()).await?;

	let (tag, body) = match identify_result {
		CacheStatus::Cached(v) => ("HIT", v),
		CacheStatus::NonCached(v) => ("MISS", v),
	};
	debug!("Cache {tag} for identify game and get metadata ids");

	Ok(HttpResponse::Ok()
		.append_header(("X-Cache", tag))
		.json(body))
}

/// Identifies a game by its file hashes or by filename and size.
///
/// The strongest supplied hash resolves the file: sha256, then sha1, then md5,
/// then crc. When no hash matches, the filename and size are tried last. The
/// result carries the matched game together with its game files, metadata
/// mappings, publisher, and company.
///
/// When the resolving hash is shared by sibling games, those co-hashed games
/// are returned in `additionalMatches`, ranked after the primary match. The
/// array is omitted for single-game results and for filename and size results.
#[utoipa::path(
	get,
	tag = "Identify",
	params(GameFileMatchSearch),
	responses(
		(status = 200, description = "The match result and its related records, including a no-match outcome", body = GameAndRelationMatchResultV2),
		(status = 400, description = "Invalid query parameters", body = V2ErrorBody)
	)
)]
#[get("/identify/relations")]
pub async fn identify_game_and_relations_v2(
	query: Query<GameFileMatchSearch>,
	db_conn: Data<DatabaseConnection>,
	redis_conn: Data<MultiplexedConnection>,
) -> error::Result<impl Responder> {
	let query = query.into_inner();
	if let Err(msg) = query.validate() {
		return Ok(v2_bad_request("invalid_query", msg));
	}

	let mut redis_conn = redis_conn.get_ref().clone();
	let identify_result =
		identify_game_and_get_relations_v2(query, &mut redis_conn, db_conn.get_ref()).await?;

	let (tag, body) = match identify_result {
		CacheStatus::Cached(v) => ("HIT", v),
		CacheStatus::NonCached(v) => ("MISS", v),
	};
	debug!("Cache {tag} for identify game and relations");

	Ok(HttpResponse::Ok()
		.append_header(("X-Cache", tag))
		.json(body))
}
