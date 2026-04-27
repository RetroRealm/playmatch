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

/// List every platform MobyGames knows about.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "MobyGames",
	responses(
		(status = 200, description = "MobyGames platform catalog", body = Vec<MgPlatform>)
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

/// List every genre MobyGames knows about.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "MobyGames",
	responses(
		(status = 200, description = "MobyGames genre catalog", body = Vec<MgGenre>)
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

/// Look up a MobyGames game by its game id.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "MobyGames",
	params(MgIdQuery),
	responses(
		(status = 200, description = "MobyGames game record", body = MgGame),
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

/// Search MobyGames games by title, optionally narrowed to a platform.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "MobyGames",
	params(MgSearchQuery),
	responses(
		(status = 200, description = "Matching games", body = Vec<MgGame>)
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

/// Fetch MobyGames cover groups for a given game/platform combination.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "MobyGames",
	params(MgGamePlatformQuery),
	responses(
		(status = 200, description = "MobyGames covers", body = MgCoversResp)
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

/// Fetch MobyGames screenshots for a given game/platform combination.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "MobyGames",
	params(MgGamePlatformQuery),
	responses(
		(status = 200, description = "MobyGames screenshots", body = MgScreenshotsResp)
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
