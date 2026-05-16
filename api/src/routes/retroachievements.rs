use crate::error;
use crate::model::igdb::validate_search_literal;
use crate::model::retroachievements::{RaHashQuery, RaIdQuery, RaSearchQuery};
use actix_web::web::Data;
use actix_web::{HttpResponse, Responder, get};
use actix_web_lab::extract::Query;
use redis::aio::MultiplexedConnection;
use sea_orm::DatabaseConnection;
use service::providers::retroachievements::cache::{
	get_ra_game_by_id_cached, get_ra_game_by_md5_cached, get_ra_systems_cached,
	search_ra_games_cached,
};
#[allow(unused_imports)] // Referenced only inside utoipa::path body attributes.
use service::providers::retroachievements::model::{RaGame, RaGameHash, RaGameMatch, RaSystem};

/// List every console RetroAchievements knows about.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "RetroAchievements",
	responses(
		(status = 200, description = "RetroAchievements system catalog", body = Vec<RaSystem>)
	)
)]
#[get("/retroachievements/systems")]
pub async fn list_ra_systems(
	db_conn: Data<DatabaseConnection>,
	redis_conn: Data<MultiplexedConnection>,
) -> error::Result<impl Responder> {
	let mut redis_conn = redis_conn.get_ref().clone();
	let systems = get_ra_systems_cached(db_conn.get_ref(), &mut redis_conn).await?;
	Ok(HttpResponse::Ok().json(systems))
}

/// Look up a RetroAchievements game by its game id.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "RetroAchievements",
	params(RaIdQuery),
	responses(
		(status = 200, description = "RetroAchievements game record", body = RaGame),
		(status = 404, description = "Game not found")
	)
)]
#[get("/retroachievements/game")]
pub async fn get_ra_game_by_id(
	query: Query<RaIdQuery>,
	db_conn: Data<DatabaseConnection>,
	redis_conn: Data<MultiplexedConnection>,
) -> error::Result<impl Responder> {
	let mut redis_conn = redis_conn.get_ref().clone();
	let response =
		get_ra_game_by_id_cached(db_conn.get_ref(), &mut redis_conn, query.into_inner().id).await?;
	Ok(match response {
		Some(game) => HttpResponse::Ok().json(game),
		None => HttpResponse::NotFound().finish(),
	})
}

/// Look up a RetroAchievements game by one of its MD5 hashes and return the
/// game record together with every other hash RetroAchievements has for it.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "RetroAchievements",
	params(RaHashQuery),
	responses(
		(status = 200, description = "Matched game and its hashes", body = RaGameMatch),
		(status = 404, description = "No game matched the supplied hash")
	)
)]
#[get("/retroachievements/game/by-hash")]
pub async fn get_ra_game_by_hash(
	query: Query<RaHashQuery>,
	db_conn: Data<DatabaseConnection>,
	redis_conn: Data<MultiplexedConnection>,
) -> error::Result<impl Responder> {
	let mut redis_conn = redis_conn.get_ref().clone();
	let response =
		get_ra_game_by_md5_cached(db_conn.get_ref(), &mut redis_conn, query.into_inner().md5)
			.await?;
	Ok(match response {
		Some(payload) => HttpResponse::Ok().json(payload),
		None => HttpResponse::NotFound().finish(),
	})
}

/// Search RetroAchievements games by title, optionally narrowed to a system.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "RetroAchievements",
	params(RaSearchQuery),
	responses(
		(status = 200, description = "Matching games", body = Vec<RaGame>)
	)
)]
#[get("/retroachievements/game/search")]
pub async fn search_ra_games(
	query: Query<RaSearchQuery>,
	db_conn: Data<DatabaseConnection>,
	redis_conn: Data<MultiplexedConnection>,
) -> error::Result<impl Responder> {
	let q = query.into_inner();
	if let Err(resp) = validate_search_literal(&q.query) {
		return Ok(resp);
	}
	let response = search_ra_games_cached(
		db_conn.get_ref(),
		&mut redis_conn.get_ref().clone(),
		q.system_name,
		q.query,
	)
	.await?;
	Ok(HttpResponse::Ok().json(response))
}
