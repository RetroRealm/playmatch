use crate::error;
use crate::model::igdb::{IdQuery, IdsQuery, SearchQuery};
use crate::util::igdb_route_mutli_id_helper;
use actix_web::web::Data;
use actix_web::{get, HttpResponse, Responder};
use actix_web_lab::extract::Query;
use service::cache::igdb::{
	get_age_rating_by_id_cached, get_alternative_name_by_id_cached, get_artwork_by_id_cached,
	get_collection_by_id_cached, get_cover_by_id_cached, get_external_game_by_id_cached,
	get_franchise_by_id_cached, get_game_by_id_cached, get_genre_by_id_cached,
	search_game_by_name_cached,
};
use service::metadata::igdb::model::{
	AgeRating, AlternativeName, Artwork, Collection, Cover, ExternalGame, Franchise, Game, Genre,
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
	let response = igdb_route_mutli_id_helper::<Game>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			async move { get_game_by_id_cached(client.as_ref(), id).await }
		})
	})
	.await?;

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
	let response = igdb_route_mutli_id_helper::<AgeRating>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			async move { get_age_rating_by_id_cached(client.as_ref(), id).await }
		})
	})
	.await?;

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
	let response = igdb_route_mutli_id_helper::<AlternativeName>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			async move { get_alternative_name_by_id_cached(client.as_ref(), id).await }
		})
	})
	.await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for an Artwork by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about an Artwork", body = Artwork),
		(status = 404, description = "Artwork not found")
	)
)]
#[get("/igdb/artwork")]
pub async fn get_artwork_by_id(
	query: Query<IdQuery>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_artwork_by_id_cached(igdb_client.as_ref(), query.into_inner().id).await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for Artworks by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about Artworks", body = Vec<Artwork>)
	)
)]
#[get("/igdb/artworks")]
pub async fn get_artworks_by_ids(
	query: Query<IdsQuery>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = igdb_route_mutli_id_helper::<Artwork>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			async move { get_artwork_by_id_cached(client.as_ref(), id).await }
		})
	})
	.await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for an Collection by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about an Collection", body = Collection),
		(status = 404, description = "Collection not found")
	)
)]
#[get("/igdb/collection")]
pub async fn get_collection_by_id(
	query: Query<IdQuery>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_collection_by_id_cached(igdb_client.as_ref(), query.into_inner().id).await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for Collections by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about Collections", body = Vec<Collection>)
	)
)]
#[get("/igdb/collections")]
pub async fn get_collections_by_ids(
	query: Query<IdsQuery>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = igdb_route_mutli_id_helper::<Collection>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			async move { get_collection_by_id_cached(client.as_ref(), id).await }
		})
	})
	.await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for an Cover by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about an Cover", body = Cover),
		(status = 404, description = "Cover not found")
	)
)]
#[get("/igdb/cover")]
pub async fn get_cover_by_id(
	query: Query<IdQuery>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_cover_by_id_cached(igdb_client.as_ref(), query.into_inner().id).await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for Covers by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about Covers", body = Vec<Cover>)
	)
)]
#[get("/igdb/covers")]
pub async fn get_covers_by_ids(
	query: Query<IdsQuery>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = igdb_route_mutli_id_helper::<Cover>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			async move { get_cover_by_id_cached(client.as_ref(), id).await }
		})
	})
	.await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for an External Game by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about an External Game", body = ExternalGame),
		(status = 404, description = "External Game not found")
	)
)]
#[get("/igdb/external-game")]
pub async fn get_external_game_by_id(
	query: Query<IdQuery>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response =
		get_external_game_by_id_cached(igdb_client.as_ref(), query.into_inner().id).await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for External Games by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about External Games", body = Vec<ExternalGame>)
	)
)]
#[get("/igdb/external-games")]
pub async fn get_external_games_by_ids(
	query: Query<IdsQuery>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = igdb_route_mutli_id_helper::<ExternalGame>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			async move { get_external_game_by_id_cached(client.as_ref(), id).await }
		})
	})
	.await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Franchise by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about an Franchise", body = Franchise),
		(status = 404, description = "Franchise not found")
	)
)]
#[get("/igdb/franchise")]
pub async fn get_franchise_by_id(
	query: Query<IdQuery>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_franchise_by_id_cached(igdb_client.as_ref(), query.into_inner().id).await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for Franchise by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about Franchise", body = Vec<Franchise>)
	)
)]
#[get("/igdb/franchises")]
pub async fn get_franchises_by_ids(
	query: Query<IdsQuery>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = igdb_route_mutli_id_helper::<Franchise>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			async move { get_franchise_by_id_cached(client.as_ref(), id).await }
		})
	})
	.await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Genre by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a Genre", body = Genre),
		(status = 404, description = "Genre not found")
	)
)]
#[get("/igdb/genre")]
pub async fn get_genre_by_id(
	query: Query<IdQuery>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_genre_by_id_cached(igdb_client.as_ref(), query.into_inner().id).await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for Genres by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about Genres", body = Vec<Genre>)
	)
)]
#[get("/igdb/genres")]
pub async fn get_genres_by_ids(
	query: Query<IdsQuery>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = igdb_route_mutli_id_helper::<Genre>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			async move { get_genre_by_id_cached(client.as_ref(), id).await }
		})
	})
	.await?;

	Ok(HttpResponse::Ok().json(response))
}
