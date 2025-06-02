use crate::error;
use actix_web::web::{Data, Json};
use actix_web::{post, HttpResponse, Responder};
use sea_orm::DatabaseConnection;
use service::game::apply_manual_game_match;
use service::model::MatchRequest;
use std::env;

/// Endpoint is currently private as community suggestions are still being worked on, manually match a game by its file hashes or filename, returning the matched game ExternalMetadata.
#[utoipa::path(
	post,
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
	req: actix_web::HttpRequest,
) -> error::Result<impl Responder> {
	let auth_header = req
		.headers()
		.get("Authorization")
		.map(|h| h.to_str().ok())
		.flatten();

	if auth_header.is_none() {
		return Ok(HttpResponse::Unauthorized().body("Authorization header is required."));
	}

	let auth_header = auth_header.unwrap();

	if !auth_header.starts_with("Bearer ") {
		return Ok(
			HttpResponse::Unauthorized().body("Authorization header must start with 'Bearer '.")
		);
	}

	let token = &auth_header[7..]; // Remove "Bearer " prefix

	if token.is_empty() {
		return Ok(HttpResponse::Unauthorized().body("Token is required."));
	}

	let expected_token =
		env::var("API_MATCH_ENDPOINT_AUTHORIZATION").unwrap_or_else(|_| "".to_string());

	if !expected_token.eq(token) {
		return Ok(HttpResponse::Unauthorized().body("Invalid API token."));
	}

	if match_request.name.is_none()
		&& match_request.md5.is_none()
		&& match_request.sha1.is_none()
		&& match_request.sha256.is_none()
	{
		return Ok(HttpResponse::BadRequest()
			.body("At least one of file_name, md5, sha1 or sha256 must be provided."));
	}

	let updated = apply_manual_game_match(match_request.into_inner(), db_conn.get_ref()).await?;

	Ok(HttpResponse::Ok().json(updated))
}
