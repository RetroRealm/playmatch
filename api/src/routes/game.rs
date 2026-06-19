use crate::error;
use crate::model::game::GameSearchQuery;
use crate::model::igdb::validate_search_literal;
use actix_web::web::{Data, Query};
use actix_web::{HttpResponse, Responder, get, web};
use sea_orm::DatabaseConnection;
#[allow(unused_imports)] // Referenced only inside the utoipa::path body attribute.
use service::entities::game::search_games_by_name_and_platform;
use service::identification::{
	get_game_and_all_relations, get_game_by_id_from_db, get_game_file_history,
};
#[allow(unused_imports)] // Referenced only inside the utoipa::path body attribute.
use service::model::GameNameSearchResult;
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

/// Fuzzy-searches the game catalogue by human title, optionally narrowed to a
/// platform. Returns candidate games ordered by relevance.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "Game",
	params(GameSearchQuery),
	responses(
		(status = 200, description = "Matching games ordered by relevance", body = Vec<GameNameSearchResult>),
		(status = 400, description = "The search query was empty or too long")
	)
)]
#[get("/games/search")]
pub async fn search_games(
	query: Query<GameSearchQuery>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let query = query.into_inner();
	if let Err(resp) = validate_search_literal(&query.query) {
		return Ok(resp);
	}

	let results = search_games_by_name_and_platform(
		query.query.trim(),
		query.platform_id,
		query.limit,
		db_conn.get_ref(),
	)
	.await?;

	Ok(HttpResponse::Ok().json(results))
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
