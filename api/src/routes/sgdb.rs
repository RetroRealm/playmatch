use crate::error;
use crate::model::igdb::validate_search_literal;
use crate::model::sgdb::{
	SgdbGameAssetQuery, SgdbIdQuery, SgdbPlatformAssetQuery, SgdbPlatformQuery, SgdbSearchQuery,
};
use actix_web::web::Data;
use actix_web::{HttpResponse, Responder, get};
use actix_web_lab::extract::Query;
use redis::aio::MultiplexedConnection;
use service::providers::steamgriddb::SteamGridDbClient;
use service::providers::steamgriddb::cache::{
	get_sgdb_game_by_id_cached, get_sgdb_game_by_platform_cached, get_sgdb_grids_by_game_cached,
	get_sgdb_grids_by_platform_cached, get_sgdb_heroes_by_game_cached,
	get_sgdb_heroes_by_platform_cached, get_sgdb_icons_by_game_cached,
	get_sgdb_icons_by_platform_cached, get_sgdb_logos_by_game_cached,
	get_sgdb_logos_by_platform_cached, search_sgdb_games_cached,
};
#[allow(unused_imports)] // Referenced only inside utoipa::path body attributes.
use service::providers::steamgriddb::model::{SgdbAsset, SgdbGame};

/// Look up a SteamGridDB game by its SGDB id.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "SteamGridDB",
	params(SgdbIdQuery),
	responses(
		(status = 200, description = "SteamGridDB game record", body = SgdbGame),
		(status = 404, description = "Game not found")
	)
)]
#[get("/sgdb/game")]
pub async fn get_sgdb_game_by_id(
	query: Query<SgdbIdQuery>,
	redis_conn: Data<MultiplexedConnection>,
	client: Data<SteamGridDbClient>,
) -> error::Result<impl Responder> {
	let mut redis_conn = redis_conn.get_ref().clone();
	let response =
		get_sgdb_game_by_id_cached(client.as_ref(), &mut redis_conn, query.into_inner().id).await?;
	Ok(match response {
		Some(game) => HttpResponse::Ok().json(game),
		None => HttpResponse::NotFound().finish(),
	})
}

/// Look up a SteamGridDB game by an external platform id (steam, origin, egs, etc.).
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "SteamGridDB",
	params(SgdbPlatformQuery),
	responses(
		(status = 200, description = "SteamGridDB game record", body = SgdbGame),
		(status = 404, description = "Game not found")
	)
)]
#[get("/sgdb/game/by-platform")]
pub async fn get_sgdb_game_by_platform(
	query: Query<SgdbPlatformQuery>,
	redis_conn: Data<MultiplexedConnection>,
	client: Data<SteamGridDbClient>,
) -> error::Result<impl Responder> {
	let mut redis_conn = redis_conn.get_ref().clone();
	let q = query.into_inner();
	let response = get_sgdb_game_by_platform_cached(
		client.as_ref(),
		&mut redis_conn,
		q.platform,
		q.platform_id,
	)
	.await?;
	Ok(match response {
		Some(game) => HttpResponse::Ok().json(game),
		None => HttpResponse::NotFound().finish(),
	})
}

/// Search SteamGridDB games via its autocomplete endpoint.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "SteamGridDB",
	params(SgdbSearchQuery),
	responses(
		(status = 200, description = "Matching games", body = Vec<SgdbGame>)
	)
)]
#[get("/sgdb/game/search")]
pub async fn search_sgdb_games(
	query: Query<SgdbSearchQuery>,
	redis_conn: Data<MultiplexedConnection>,
	client: Data<SteamGridDbClient>,
) -> error::Result<impl Responder> {
	let q = query.into_inner().query;
	if let Err(resp) = validate_search_literal(&q) {
		return Ok(resp);
	}
	let response =
		search_sgdb_games_cached(client.as_ref(), &mut redis_conn.get_ref().clone(), q).await?;
	Ok(HttpResponse::Ok().json(response))
}

#[rustfmt::skip]
macro_rules! sgdb_assets_by_game_route {
	($route:literal, $fn_name:ident, $cached_fn:ident, $tag_summary:literal) => {
		#[utoipa::path(
							get,
							context_path = "/api",
							tag = "SteamGridDB",
							params(SgdbGameAssetQuery),
							responses(
								(status = 200, description = $tag_summary, body = Vec<SgdbAsset>)
							)
						)]
		#[get($route)]
		pub async fn $fn_name(
			query: Query<SgdbGameAssetQuery>,
			redis_conn: Data<MultiplexedConnection>,
			client: Data<SteamGridDbClient>,
		) -> error::Result<impl Responder> {
			let mut redis_conn = redis_conn.get_ref().clone();
			let q = query.into_inner();
			let response = $cached_fn(
				client.as_ref(),
				&mut redis_conn,
				q.game_id,
				q.filters.into_filters(),
			)
			.await?;
			Ok(HttpResponse::Ok().json(response))
		}
	};
}

#[rustfmt::skip]
macro_rules! sgdb_assets_by_platform_route {
	($route:literal, $fn_name:ident, $cached_fn:ident, $tag_summary:literal) => {
		#[utoipa::path(
							get,
							context_path = "/api",
							tag = "SteamGridDB",
							params(SgdbPlatformAssetQuery),
							responses(
								(status = 200, description = $tag_summary, body = Vec<SgdbAsset>)
							)
						)]
		#[get($route)]
		pub async fn $fn_name(
			query: Query<SgdbPlatformAssetQuery>,
			redis_conn: Data<MultiplexedConnection>,
			client: Data<SteamGridDbClient>,
		) -> error::Result<impl Responder> {
			let mut redis_conn = redis_conn.get_ref().clone();
			let q = query.into_inner();
			let response = $cached_fn(
				client.as_ref(),
				&mut redis_conn,
				q.platform,
				q.platform_id,
				q.filters.into_filters(),
			)
			.await?;
			Ok(HttpResponse::Ok().json(response))
		}
	};
}

sgdb_assets_by_game_route!(
	"/sgdb/grids",
	get_sgdb_grids_by_game,
	get_sgdb_grids_by_game_cached,
	"Grid assets for the requested SGDB game"
);
sgdb_assets_by_platform_route!(
	"/sgdb/grids/by-platform",
	get_sgdb_grids_by_platform,
	get_sgdb_grids_by_platform_cached,
	"Grid assets for the requested external platform id"
);

sgdb_assets_by_game_route!(
	"/sgdb/heroes",
	get_sgdb_heroes_by_game,
	get_sgdb_heroes_by_game_cached,
	"Hero assets for the requested SGDB game"
);
sgdb_assets_by_platform_route!(
	"/sgdb/heroes/by-platform",
	get_sgdb_heroes_by_platform,
	get_sgdb_heroes_by_platform_cached,
	"Hero assets for the requested external platform id"
);

sgdb_assets_by_game_route!(
	"/sgdb/logos",
	get_sgdb_logos_by_game,
	get_sgdb_logos_by_game_cached,
	"Logo assets for the requested SGDB game"
);
sgdb_assets_by_platform_route!(
	"/sgdb/logos/by-platform",
	get_sgdb_logos_by_platform,
	get_sgdb_logos_by_platform_cached,
	"Logo assets for the requested external platform id"
);

sgdb_assets_by_game_route!(
	"/sgdb/icons",
	get_sgdb_icons_by_game,
	get_sgdb_icons_by_game_cached,
	"Icon assets for the requested SGDB game"
);
sgdb_assets_by_platform_route!(
	"/sgdb/icons/by-platform",
	get_sgdb_icons_by_platform,
	get_sgdb_icons_by_platform_cached,
	"Icon assets for the requested external platform id"
);
