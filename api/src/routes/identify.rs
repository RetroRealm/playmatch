use crate::error;
use actix_web::web::Data;
use actix_web::{HttpResponse, Responder, get, web};
use sea_orm::DatabaseConnection;
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
) -> error::Result<impl Responder> {
	let response =
		identify_game_and_metadata_mappings(query.into_inner(), db_conn.get_ref()).await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Identify a game by its file hashes or filename and size, goes in order sha256, sha1, md5 and filename + size (from most accurate to least accurate), returning information about the game, game files, publisher and company
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
) -> error::Result<impl Responder> {
	let response = identify_game_and_get_relations(query.into_inner(), db_conn.get_ref()).await?;

	Ok(HttpResponse::Ok().json(response))
}
