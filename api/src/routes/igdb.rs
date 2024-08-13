use crate::error;
use crate::model::igdb::{IdQuery, IdsQuery, SearchQuery};
use actix_web::web::Data;
use actix_web::{get, HttpResponse, Responder};
use actix_web_lab::extract::Query;
use service::cache::igdb::{
	get_age_rating_by_id_cached, get_age_ratings_by_id_cached, get_alternative_name_by_id_cached,
	get_alternative_names_by_id_cached, get_game_by_id_cached, get_games_by_ids_cached,
	search_game_by_name_cached,
};
use service::metadata::igdb::IgdbClient;

/// Queries the IGDB API for a game by its Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about an game", body = Game),
		(status = 404, description = "Game not found")
	)
)]
#[get("/igdb/game")]
pub async fn get_game_by_id(
	query: Query<IdQuery>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_game_by_id_cached(igdb_client.as_ref(), query.into_inner().id).await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for games by its Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about games", body = Vec<Game>)
	)
)]
#[get("/igdb/games")]
pub async fn get_games_by_ids(
	query: Query<IdsQuery>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_games_by_ids_cached(igdb_client.as_ref(), query.into_inner().ids).await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Searches the IGDB API for games by its name
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(SearchQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about games", body = Vec<Game>)
	)
)]
#[get("/igdb/game/search")]
pub async fn search_game_by_name(
	query: Query<SearchQuery>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response =
		search_game_by_name_cached(igdb_client.as_ref(), query.into_inner().query).await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for an Age Rating by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about an age rating", body = AgeRating),
		(status = 404, description = "Age rating not found")
	)
)]
#[get("/igdb/age-rating")]
pub async fn get_age_rating_by_id(
	query: Query<IdQuery>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_age_rating_by_id_cached(igdb_client.as_ref(), query.into_inner().id).await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for Age Ratings by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about age ratings", body = Vec<AgeRating>)
	)
)]
#[get("/igdb/age-ratings")]
pub async fn get_age_ratings_by_ids(
	query: Query<IdsQuery>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response =
		get_age_ratings_by_id_cached(igdb_client.as_ref(), query.into_inner().ids).await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for an Alternative Name by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about an alternative name", body = AlternativeName),
		(status = 404, description = "Age rating not found")
	)
)]
#[get("/igdb/alternative-name")]
pub async fn get_alternative_name_by_id(
	query: Query<IdQuery>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response =
		get_alternative_name_by_id_cached(igdb_client.as_ref(), query.into_inner().id).await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for Alternative Names by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about Alternative Names", body = Vec<AlternativeName>)
	)
)]
#[get("/igdb/alternative-names")]
pub async fn get_alternative_names_by_ids(
	query: Query<IdsQuery>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response =
		get_alternative_names_by_id_cached(igdb_client.as_ref(), query.into_inner().ids).await?;

	Ok(HttpResponse::Ok().json(response))
}
