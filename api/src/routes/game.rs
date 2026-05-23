use crate::error;
use actix_web::web::Data;
use actix_web::{HttpResponse, Responder, get, web};
use sea_orm::DatabaseConnection;
use service::identification::{get_game_and_all_relations, get_game_by_id_from_db};
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
