use crate::error;
use actix_web::web::Data;
use actix_web::{get, web, HttpResponse, Responder};
use log::debug;
use sea_orm::DatabaseConnection;
use service::cache::identify::identify_game_cached;
use service::model::GameFileMatchSearch;
use web::Query;

/// Identify a game by its file hashes or filename and size, returning the matched metadata ids, goes in order sha256, sha1, md5 and filename + size (from most accurate to least accurate)
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "Identify",
	params(GameFileMatchSearch),
	responses(
		(status = 200, description = "Returns info about a possible match via hashes or filename and size", body = GameMatchResult)
	)
)]
#[get("/identify/ids")]
pub async fn identify(
	query: Query<GameFileMatchSearch>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	debug!("Received request: {:?}", query);

	let response = identify_game_cached(db_conn.get_ref(), query.into_inner()).await?;

	Ok(HttpResponse::Ok().json(response))
}
