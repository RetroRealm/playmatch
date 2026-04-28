use crate::error;
use crate::model::igdb::validate_search_literal;
use crate::model::launchbox::{LbGameIdQuery, LbIdQuery, LbSearchQuery};
use actix_web::web::Data;
use actix_web::{HttpResponse, Responder, get};
use actix_web_lab::extract::Query;
use redis::aio::MultiplexedConnection;
use sea_orm::DatabaseConnection;
use service::providers::launchbox::cache::{
	get_lb_game_alternate_names_cached, get_lb_game_by_id_cached, get_lb_game_images_cached,
	get_lb_platforms_cached, search_lb_games_cached,
};
#[allow(unused_imports)] // Referenced only inside utoipa::path body attributes.
use service::providers::launchbox::model::{LbGame, LbGameAlternateName, LbGameImage, LbPlatform};

/// List every platform LaunchBox knows about.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "LaunchBox",
	responses(
		(status = 200, description = "LaunchBox platform catalog", body = Vec<LbPlatform>)
	)
)]
#[get("/launchbox/platforms")]
pub async fn list_lb_platforms(
	db_conn: Data<DatabaseConnection>,
	redis_conn: Data<MultiplexedConnection>,
) -> error::Result<impl Responder> {
	let mut redis_conn = redis_conn.get_ref().clone();
	let platforms = get_lb_platforms_cached(db_conn.get_ref(), &mut redis_conn).await?;
	Ok(HttpResponse::Ok().json(platforms))
}

/// Look up a LaunchBox game by its database id.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "LaunchBox",
	params(LbIdQuery),
	responses(
		(status = 200, description = "LaunchBox game record", body = LbGame),
		(status = 404, description = "Game not found")
	)
)]
#[get("/launchbox/game")]
pub async fn get_lb_game_by_id(
	query: Query<LbIdQuery>,
	db_conn: Data<DatabaseConnection>,
	redis_conn: Data<MultiplexedConnection>,
) -> error::Result<impl Responder> {
	let mut redis_conn = redis_conn.get_ref().clone();
	let response =
		get_lb_game_by_id_cached(db_conn.get_ref(), &mut redis_conn, query.into_inner().id).await?;
	Ok(match response {
		Some(game) => HttpResponse::Ok().json(game),
		None => HttpResponse::NotFound().finish(),
	})
}

/// Search LaunchBox games by name, optionally narrowed to a platform.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "LaunchBox",
	params(LbSearchQuery),
	responses(
		(status = 200, description = "Matching games", body = Vec<LbGame>)
	)
)]
#[get("/launchbox/game/search")]
pub async fn search_lb_games(
	query: Query<LbSearchQuery>,
	db_conn: Data<DatabaseConnection>,
	redis_conn: Data<MultiplexedConnection>,
) -> error::Result<impl Responder> {
	let q = query.into_inner();
	if let Err(resp) = validate_search_literal(&q.query) {
		return Ok(resp);
	}
	let response = search_lb_games_cached(
		db_conn.get_ref(),
		&mut redis_conn.get_ref().clone(),
		q.platform_name,
		q.query,
	)
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Fetch every alternate name LaunchBox has for a given game.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "LaunchBox",
	params(LbGameIdQuery),
	responses(
		(status = 200, description = "Alternate names", body = Vec<LbGameAlternateName>)
	)
)]
#[get("/launchbox/game/alternate-names")]
pub async fn get_lb_game_alternate_names(
	query: Query<LbGameIdQuery>,
	db_conn: Data<DatabaseConnection>,
	redis_conn: Data<MultiplexedConnection>,
) -> error::Result<impl Responder> {
	let q = query.into_inner();
	let response = get_lb_game_alternate_names_cached(
		db_conn.get_ref(),
		&mut redis_conn.get_ref().clone(),
		q.game_id,
	)
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Fetch every image LaunchBox has for a given game.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "LaunchBox",
	params(LbGameIdQuery),
	responses(
		(status = 200, description = "Images", body = Vec<LbGameImage>)
	)
)]
#[get("/launchbox/game/images")]
pub async fn get_lb_game_images(
	query: Query<LbGameIdQuery>,
	db_conn: Data<DatabaseConnection>,
	redis_conn: Data<MultiplexedConnection>,
) -> error::Result<impl Responder> {
	let q = query.into_inner();
	let response = get_lb_game_images_cached(
		db_conn.get_ref(),
		&mut redis_conn.get_ref().clone(),
		q.game_id,
	)
	.await?;
	Ok(HttpResponse::Ok().json(response))
}
