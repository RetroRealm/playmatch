use crate::error;
use crate::model::igdb::validate_search_literal;
use crate::model::screenscraper::{SsIdQuery, SsRomQuery, SsSearchQuery};
use actix_web::web::Data;
use actix_web::{HttpResponse, Responder, get};
use actix_web_lab::extract::Query;
use redis::aio::MultiplexedConnection;
use service::providers::screenscraper::ScreenScraperClient;
use service::providers::screenscraper::cache::{
	get_ss_game_by_id_cached, get_ss_game_by_rom_name_cached, get_ss_systems_cached,
	search_ss_games_cached,
};
#[allow(unused_imports)] // Referenced only inside utoipa::path body attributes.
use service::providers::screenscraper::model::{SsGame, SsSystem};

fn bail_if_ss_quota_exhausted(client: &ScreenScraperClient) -> error::Result<()> {
	if client.is_quota_exhausted() {
		return Err(error::Error::UpstreamUnavailable {
			message: "screenscraper quota exhausted".into(),
			retry_after_secs: client.secs_until_recovery(),
		});
	}
	Ok(())
}

/// List every system (platform) ScreenScraper knows about.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "ScreenScraper",
	responses(
		(status = 200, description = "ScreenScraper system catalog", body = Vec<SsSystem>),
		(status = 503, description = "ScreenScraper quota exhausted", body = String)
	)
)]
#[get("/screenscraper/systems")]
pub async fn list_ss_systems(
	redis_conn: Data<MultiplexedConnection>,
	client: Data<ScreenScraperClient>,
) -> error::Result<impl Responder> {
	bail_if_ss_quota_exhausted(client.as_ref())?;
	let mut redis_conn = redis_conn.get_ref().clone();
	let systems = get_ss_systems_cached(client.as_ref(), &mut redis_conn).await?;
	Ok(HttpResponse::Ok().json(systems))
}

/// Look up a ScreenScraper game by its SS game id.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "ScreenScraper",
	params(SsIdQuery),
	responses(
		(status = 200, description = "ScreenScraper game record", body = SsGame),
		(status = 404, description = "Game not found"),
		(status = 503, description = "ScreenScraper quota exhausted", body = String)
	)
)]
#[get("/screenscraper/game")]
pub async fn get_ss_game_by_id(
	query: Query<SsIdQuery>,
	redis_conn: Data<MultiplexedConnection>,
	client: Data<ScreenScraperClient>,
) -> error::Result<impl Responder> {
	bail_if_ss_quota_exhausted(client.as_ref())?;
	let mut redis_conn = redis_conn.get_ref().clone();
	let response =
		get_ss_game_by_id_cached(client.as_ref(), &mut redis_conn, query.into_inner().id).await?;
	Ok(match response {
		Some(game) => HttpResponse::Ok().json(game),
		None => HttpResponse::NotFound().finish(),
	})
}

/// Search ScreenScraper games by name within a system.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "ScreenScraper",
	params(SsSearchQuery),
	responses(
		(status = 200, description = "Matching games", body = Vec<SsGame>),
		(status = 503, description = "ScreenScraper quota exhausted", body = String)
	)
)]
#[get("/screenscraper/game/search")]
pub async fn search_ss_games(
	query: Query<SsSearchQuery>,
	redis_conn: Data<MultiplexedConnection>,
	client: Data<ScreenScraperClient>,
) -> error::Result<impl Responder> {
	bail_if_ss_quota_exhausted(client.as_ref())?;
	let q = query.into_inner();
	if let Err(resp) = validate_search_literal(&q.query) {
		return Ok(resp);
	}
	let response = search_ss_games_cached(
		client.as_ref(),
		&mut redis_conn.get_ref().clone(),
		q.system_id,
		q.query,
	)
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Look up a ScreenScraper game by an exact rom file name within a system.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "ScreenScraper",
	params(SsRomQuery),
	responses(
		(status = 200, description = "ScreenScraper game record", body = SsGame),
		(status = 404, description = "Game not found"),
		(status = 503, description = "ScreenScraper quota exhausted", body = String)
	)
)]
#[get("/screenscraper/game/by-rom")]
pub async fn get_ss_game_by_rom_name(
	query: Query<SsRomQuery>,
	redis_conn: Data<MultiplexedConnection>,
	client: Data<ScreenScraperClient>,
) -> error::Result<impl Responder> {
	bail_if_ss_quota_exhausted(client.as_ref())?;
	let q = query.into_inner();
	let response = get_ss_game_by_rom_name_cached(
		client.as_ref(),
		&mut redis_conn.get_ref().clone(),
		q.system_id,
		q.rom_name,
	)
	.await?;
	Ok(match response {
		Some(game) => HttpResponse::Ok().json(game),
		None => HttpResponse::NotFound().finish(),
	})
}
