use crate::error;
use crate::model::igdb::GameIdQuery;
use actix_web::web::{Data, Query};
use actix_web::{get, HttpResponse, Responder};
use log::debug;
use service::metadata::igdb::IgdbClient;
use tokio::sync::Mutex;

/// Queries the IGDB API for a game by its ID
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
pub async fn get_game(
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
