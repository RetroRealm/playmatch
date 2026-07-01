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

/// Returns a game by id.
#[utoipa::path(
	get,
	tag = "Game",
	responses(
		(status = 200, description = "The matched game and its metadata", body = GameMetadataResponse)
	)
)]
#[get("/game/{id}")]
pub async fn get_playmatch_game_by_id(
	id: web::Path<Uuid>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	Ok(HttpResponse::Ok().json(get_game_by_id_from_db(id.into_inner(), db_conn.get_ref()).await?))
}

/// Returns a game by id with all related records.
///
/// The response carries the game together with its game files, metadata
/// mappings, publisher, and company.
#[utoipa::path(
	get,
	tag = "Game",
	responses(
		(status = 200, description = "The matched game and its related records", body = GameAndRelationsResult)
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

/// Searches games by name, ordered by relevance.
///
/// Pass a platform to limit the results to that platform. Returns an empty
/// array when nothing matches.
#[utoipa::path(
	get,
	tag = "Game",
	params(GameSearchQuery),
	responses(
		(status = 200, description = "Matching games ordered by relevance", body = Vec<GameNameSearchResult>),
		(status = 400, description = "Empty search query, or a query over the length limit")
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

/// Lists the dat file imports a game file was seen in, newest first.
///
/// Each entry is a dat file release in which this hash appeared.
#[utoipa::path(
	get,
	tag = "Game",
	responses(
		(status = 200, description = "The dat file imports this hash was seen in, newest first", body = Vec<PlaymatchDatFileImport>)
	)
)]
#[get("/game-file/{id}/history")]
pub async fn get_game_file_history_by_id(
	id: web::Path<Uuid>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	Ok(HttpResponse::Ok().json(get_game_file_history(id.into_inner(), db_conn.get_ref()).await?))
}
