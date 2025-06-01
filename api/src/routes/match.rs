use crate::error;
use actix_web::web::{Data, Json};
use actix_web::{post, HttpResponse, Responder};
use sea_orm::DatabaseConnection;
use service::game::apply_manual_game_match;
use service::model::{GameMatchResult, MatchRequest};

/// Manually match a game by its file hashes or filename, returning the matched game metadata. If the match is done by the Community its not directly applied but instead its saved and has to be approved by an Admin
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "Match",
	responses(
		(status = 200, description = "Successfully Matched", body = GameMatchResult),
		(status = 404, description = "Game not found")
	)
)]
#[post("/match")]
pub async fn match_game(
	match_request: Json<MatchRequest>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	if match_request.file_name.is_none()
		&& match_request.md5.is_none()
		&& match_request.sha1.is_none()
		&& match_request.sha256.is_none()
	{
		return Ok(HttpResponse::BadRequest()
			.body("At least one of file_name, md5, sha1 or sha256 must be provided."));
	}

	apply_manual_game_match(match_request.into_inner(), db_conn.get_ref()).await?;

	Ok(HttpResponse::NoContent().finish())
}
