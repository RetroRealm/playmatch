use crate::error;
use crate::model::igdb::validate_search_literal;
use crate::model::mobygames::{MgGamePlatformQuery, MgIdQuery, MgSearchQuery};
use actix_web::web::Data;
use actix_web::{HttpResponse, Responder, get};
use actix_web_lab::extract::Query;
use redis::aio::MultiplexedConnection;
use service::providers::mobygames::MobyGamesClient;
use service::providers::mobygames::cache::{
	get_mg_game_by_id_cached, get_mg_game_platform_covers_cached,
	get_mg_game_platform_screenshots_cached, get_mg_genres_cached, get_mg_platforms_cached,
	search_mg_games_cached,
};
#[allow(unused_imports)] // Referenced only inside utoipa::path body attributes.
use service::providers::mobygames::model::{
	MgCoversResp, MgGame, MgGenre, MgPlatform, MgScreenshotsResp,
};

/// Lists every platform known to MobyGames.
#[utoipa::path(
	get,
	tag = "MobyGames",
	responses(
		(status = 200, description = "The full MobyGames platform catalog", body = Vec<MgPlatform>)
	)
)]
#[get("/mobygames/platforms")]
pub async fn list_mg_platforms(
	redis_conn: Data<MultiplexedConnection>,
	client: Data<MobyGamesClient>,
) -> error::Result<impl Responder> {
	let mut redis_conn = redis_conn.get_ref().clone();
	let platforms = get_mg_platforms_cached(client.as_ref(), &mut redis_conn).await?;
	Ok(HttpResponse::Ok().json(platforms))
}

/// Lists every genre known to MobyGames.
#[utoipa::path(
	get,
	tag = "MobyGames",
	responses(
		(status = 200, description = "The full MobyGames genre catalog", body = Vec<MgGenre>)
	)
)]
#[get("/mobygames/genres")]
pub async fn list_mg_genres(
	redis_conn: Data<MultiplexedConnection>,
	client: Data<MobyGamesClient>,
) -> error::Result<impl Responder> {
	let mut redis_conn = redis_conn.get_ref().clone();
	let genres = get_mg_genres_cached(client.as_ref(), &mut redis_conn).await?;
	Ok(HttpResponse::Ok().json(genres))
}

/// Returns a MobyGames game by id.
#[utoipa::path(
	get,
	tag = "MobyGames",
	params(MgIdQuery),
	responses(
		(status = 200, description = "The matched MobyGames game record", body = MgGame),
		(status = 404, description = "Game not found")
	)
)]
#[get("/mobygames/game")]
pub async fn get_mg_game_by_id(
	query: Query<MgIdQuery>,
	redis_conn: Data<MultiplexedConnection>,
	client: Data<MobyGamesClient>,
) -> error::Result<impl Responder> {
	let mut redis_conn = redis_conn.get_ref().clone();
	let response =
		get_mg_game_by_id_cached(client.as_ref(), &mut redis_conn, query.into_inner().id).await?;
	Ok(match response {
		Some(game) => HttpResponse::Ok().json(game),
		None => HttpResponse::NotFound().finish(),
	})
}

/// Searches MobyGames games by title.
///
/// Narrow the search to a single platform by passing a platform id. If omitted, all platforms are included.
#[utoipa::path(
	get,
	tag = "MobyGames",
	params(MgSearchQuery),
	responses(
		(status = 200, description = "The matching games", body = Vec<MgGame>)
	)
)]
#[get("/mobygames/game/search")]
pub async fn search_mg_games(
	query: Query<MgSearchQuery>,
	redis_conn: Data<MultiplexedConnection>,
	client: Data<MobyGamesClient>,
) -> error::Result<impl Responder> {
	let q = query.into_inner();
	if let Err(resp) = validate_search_literal(&q.query) {
		return Ok(resp);
	}
	let response = search_mg_games_cached(
		client.as_ref(),
		&mut redis_conn.get_ref().clone(),
		q.platform_id,
		q.query,
	)
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Returns the cover groups for a game on a platform.
#[utoipa::path(
	get,
	tag = "MobyGames",
	params(MgGamePlatformQuery),
	responses(
		(status = 200, description = "The cover groups for the game on the platform", body = MgCoversResp)
	)
)]
#[get("/mobygames/game/covers")]
pub async fn get_mg_game_covers(
	query: Query<MgGamePlatformQuery>,
	redis_conn: Data<MultiplexedConnection>,
	client: Data<MobyGamesClient>,
) -> error::Result<impl Responder> {
	let q = query.into_inner();
	let response = get_mg_game_platform_covers_cached(
		client.as_ref(),
		&mut redis_conn.get_ref().clone(),
		q.game_id,
		q.platform_id,
	)
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Returns the screenshots for a game on a platform.
#[utoipa::path(
	get,
	tag = "MobyGames",
	params(MgGamePlatformQuery),
	responses(
		(status = 200, description = "The screenshots for the game on the platform", body = MgScreenshotsResp)
	)
)]
#[get("/mobygames/game/screenshots")]
pub async fn get_mg_game_screenshots(
	query: Query<MgGamePlatformQuery>,
	redis_conn: Data<MultiplexedConnection>,
	client: Data<MobyGamesClient>,
) -> error::Result<impl Responder> {
	let q = query.into_inner();
	let response = get_mg_game_platform_screenshots_cached(
		client.as_ref(),
		&mut redis_conn.get_ref().clone(),
		q.game_id,
		q.platform_id,
	)
	.await?;
	Ok(HttpResponse::Ok().json(response))
}
