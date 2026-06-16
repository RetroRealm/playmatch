use crate::error;
use actix_web::web::Data;
use actix_web::{HttpResponse, Responder, get, web};
use sea_orm::DatabaseConnection;
use service::identification::{
	get_game_and_all_relations, get_game_by_id_from_db, get_game_file_history,
};
use uuid::Uuid;

/// Gets a Playmatch game by its ID.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "Game",
	responses(
		(status = 200, description = "Returns the found game", body = GameMetadataResponse)
	)
)]
#[get("/game/{id}")]
pub async fn get_playmatch_game_by_id(
	id: web::Path<Uuid>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	Ok(HttpResponse::Ok().json(get_game_by_id_from_db(id.into_inner(), db_conn.get_ref()).await?))
}

/// Gets a Playmatch game by its ID, includes all relations.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "Game",
	responses(
		(status = 200, description = "Returns the found game including all relations", body = GameAndRelationsResult)
	)
)]
#[get("/game/{id}/with-relations")]
pub async fn get_playmatch_game_with_relations_by_id(
	id: web::Path<Uuid>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	Ok(HttpResponse::Ok()
		.json(get_game_and_all_relations(id.into_inner(), db_conn.get_ref()).await?))
}

/// Gets the dat file version history for a game file: every dat file release in
/// which this hash was seen, newest first.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "Game",
	responses(
		(status = 200, description = "Returns the dat file imports this hash was seen in, newest first", body = Vec<PlaymatchDatFileImport>)
	)
)]
#[get("/game-file/{id}/history")]
pub async fn get_game_file_history_by_id(
	id: web::Path<Uuid>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	Ok(HttpResponse::Ok().json(get_game_file_history(id.into_inner(), db_conn.get_ref()).await?))
}
