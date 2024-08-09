use crate::error;
use crate::model::game_file::{GameFileRequest, GameMatchResponse};
use actix_web::web::Data;
use actix_web::{get, web, HttpResponse, Responder};
use log::debug;
use sea_orm::DatabaseConnection;
use service::game::match_game_if_possible;
use web::Query;

/// Identify a game by its file hashes or filename and size, goes in order Sha256, Sha1, Md5, Filename and Size (from most accurate to least accurate)
#[utoipa::path(
	get,
	context_path = "/api",
	params(GameFileRequest),
	responses(
		(status = 200, description = "Returns info about a possible match via hashes or filename and size", body = GameMatchResponse)
	)
)]
#[get("/identify/ids")]
pub async fn identify(
	query: Query<GameFileRequest>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	debug!("Received request: {:?}", query);

	let response: GameMatchResponse =
		match_game_if_possible(query.into_inner().into(), db_conn.get_ref())
			.await?
			.into();

	Ok(HttpResponse::Ok().json(response))
}
