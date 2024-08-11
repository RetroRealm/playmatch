use crate::error;
use crate::model::igdb::{GameIdQuery, GameIdsQuery, GameSearchQuery};
use actix_web::web::Data;
use actix_web::{get, HttpResponse, Responder};
use actix_web_lab::extract::Query;
use log::debug;
use service::metadata::igdb::IgdbClient;
use tokio::sync::Mutex;

/// Queries the IGDB API for a game by its Id
#[utoipa::path(
	get,
	context_path = "/api",
	params(GameIdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about an game", body = Game),
		(status = 404, description = "Game not found")
	)
)]
#[get("/igdb/game")]
pub async fn get_game_by_id(
	query: Query<GameIdQuery>,
	igdb_client: Data<Mutex<IgdbClient>>,
) -> error::Result<impl Responder> {
	debug!("Received request: {:?}", query);

	let mut guard = igdb_client.lock().await;

	let response = guard.get_game_by_id(query.into_inner().id).await?;

	drop(guard);

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for games by its Ids
#[utoipa::path(
	get,
	context_path = "/api",
	params(GameIdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about games", body = Vec<Game>)
	)
)]
#[get("/igdb/games")]
pub async fn get_games_by_ids(
	query: Query<GameIdsQuery>,
	igdb_client: Data<Mutex<IgdbClient>>,
) -> error::Result<impl Responder> {
	debug!("Received request: {:?}", query);

	let mut guard = igdb_client.lock().await;

	let response = guard.get_games_by_id(query.into_inner().ids).await?;

	drop(guard);

	Ok(HttpResponse::Ok().json(response))
}

/// Searches the IGDB API for games by its name
#[utoipa::path(
	get,
	context_path = "/api",
	params(GameSearchQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about games", body = Vec<Game>)
	)
)]
#[get("/igdb/game/search")]
pub async fn search_game_by_name(
	query: Query<GameSearchQuery>,
	igdb_client: Data<Mutex<IgdbClient>>,
) -> error::Result<impl Responder> {
	debug!("Received request: {:?}", query);

	let mut guard = igdb_client.lock().await;

	let response = guard.search_game_by_name(query.into_inner().query).await?;

	drop(guard);

	Ok(HttpResponse::Ok().json(response))
}
