use crate::error;
use crate::model::igdb::{IdQuery, IdsQuery, SearchQuery, SlugIdQuery};
use crate::util::igdb_route_mutli_id_helper;
use actix_web::web::Data;
use actix_web::{HttpResponse, Responder, get};
use actix_web_lab::extract::Query;
use service::cache::igdb::{
	get_age_rating_by_id_cached, get_age_rating_category_by_id_cached,
	get_age_rating_content_description_type_by_id_cached,
	get_age_rating_content_description_v2_by_id_cached, get_age_rating_organization_by_id_cached,
	get_alternative_name_by_id_cached, get_artwork_by_id_cached, get_artwork_type_by_id_cached,
	get_character_mug_shot_by_id_cached, get_collection_by_id_cached,
	get_company_size_by_id_cached, get_company_status_by_id_cached, get_company_type_by_id_cached,
	get_company_type_history_by_id_cached, get_cover_by_id_cached, get_date_format_by_id_cached,
	get_entity_type_by_id_cached, get_external_game_by_id_cached,
	get_external_game_source_by_id_cached, get_franchise_by_id_cached, get_game_by_id_cached,
	get_game_by_slug_cached, get_game_release_format_by_id_cached, get_game_status_by_id_cached,
	get_game_time_to_beat_by_id_cached, get_game_type_by_id_cached, get_genre_by_id_cached,
	get_platform_type_by_id_cached, get_release_date_region_by_id_cached, get_report_by_id_cached,
	get_report_type_by_id_cached, get_website_type_by_id_cached, search_game_by_name_cached,
};
use service::metadata::igdb::IgdbClient;
use service::metadata::igdb::model::{
	AgeRating, AgeRatingCategory, AgeRatingContentDescriptionType, AgeRatingContentDescriptionV2,
	AgeRatingOrganization, AlternativeName, Artwork, ArtworkType, CharacterMugShot, Collection,
	CompanySize, CompanyStatus, CompanyType, CompanyTypeHistory, Cover, DateFormat, EntityType,
	ExternalGame, ExternalGameSource, Franchise, Game, GameReleaseFormat, GameStatus,
	GameTimeToBeat, GameType, Genre, PlatformType, ReleaseDateRegion, Report, ReportType,
	WebsiteType,
};

/// Queries the IGDB API for a game by its Id or Slug
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(SlugIdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about an game", body = Game),
		(status = 404, description = "Game not found")
	)
)]
#[get("/igdb/game")]
pub async fn get_igdb_game_by_id(
	query: Query<SlugIdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let query = query.into_inner();
	let redis_conn = &mut redis_client.get_multiplexed_async_connection().await?;

	let response = if let Some(id) = query.id {
		get_game_by_id_cached(igdb_client.as_ref(), redis_conn, id).await?
	} else if let Some(slug) = query.slug {
		get_game_by_slug_cached(igdb_client.as_ref(), redis_conn, slug).await?
	} else {
		return Ok(HttpResponse::BadRequest().body("Either slug or id must be provided"));
	};

	if let Some(game) = response {
		Ok(HttpResponse::Ok().json(game))
	} else {
		Ok(HttpResponse::NotFound().finish())
	}
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
pub async fn get_igdb_games_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;

	let response = igdb_route_mutli_id_helper::<Game>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_game_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
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
pub async fn search_igdb_game_by_name(
	query: Query<SearchQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = search_game_by_name_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().query,
	)
	.await?;

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
pub async fn get_igdb_age_rating_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_age_rating_by_id_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().id,
	)
	.await?;

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
pub async fn get_igdb_age_ratings_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;

	let response = igdb_route_mutli_id_helper::<AgeRating>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_age_rating_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
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
pub async fn get_igdb_alternative_name_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_alternative_name_by_id_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().id,
	)
	.await?;

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
pub async fn get_igdb_alternative_names_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<AlternativeName>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_alternative_name_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
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
pub async fn get_igdb_artwork_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_artwork_by_id_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().id,
	)
	.await?;

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
pub async fn get_igdb_artworks_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;

	let response = igdb_route_mutli_id_helper::<Artwork>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_artwork_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
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
pub async fn get_igdb_collection_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_collection_by_id_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().id,
	)
	.await?;

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
pub async fn get_igdb_collections_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;

	let response = igdb_route_mutli_id_helper::<Collection>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_collection_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
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
pub async fn get_igdb_cover_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_cover_by_id_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().id,
	)
	.await?;

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
pub async fn get_igdb_covers_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;

	let response = igdb_route_mutli_id_helper::<Cover>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_cover_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
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
pub async fn get_igdb_external_game_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_external_game_by_id_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().id,
	)
	.await?;

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
pub async fn get_igdb_external_games_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;

	let response = igdb_route_mutli_id_helper::<ExternalGame>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_external_game_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
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
pub async fn get_igdb_franchise_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_franchise_by_id_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().id,
	)
	.await?;

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
pub async fn get_igdb_franchises_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;

	let response = igdb_route_mutli_id_helper::<Franchise>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_franchise_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
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
pub async fn get_igdb_genre_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_genre_by_id_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().id,
	)
	.await?;

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
pub async fn get_igdb_genres_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;

	let response = igdb_route_mutli_id_helper::<Genre>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_genre_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for an Age Rating Category by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about an Age Rating Category", body = AgeRatingCategory),
		(status = 404, description = "Age Rating Category not found")
	)
)]
#[get("/igdb/age-rating-category")]
pub async fn get_igdb_age_rating_category_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_age_rating_category_by_id_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().id,
	)
	.await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for Age Rating Categories by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about Age Rating Categories", body = Vec<AgeRatingCategory>)
	)
)]
#[get("/igdb/age-rating-categories")]
pub async fn get_igdb_age_rating_categories_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<AgeRatingCategory>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move {
				get_age_rating_category_by_id_cached(client.as_ref(), &mut redis_conn, id).await
			}
		})
	})
	.await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for an Age Rating Content Description (V2) by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about an Age Rating Content Description", body = AgeRatingContentDescriptionV2),
		(status = 404, description = "Age Rating Content Description not found")
	)
)]
#[get("/igdb/age-rating-content-description-v2")]
pub async fn get_igdb_age_rating_content_description_v2_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_age_rating_content_description_v2_by_id_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().id,
	)
	.await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for Age Rating Content Descriptions (V2) by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about Age Rating Content Descriptions", body = Vec<AgeRatingContentDescriptionV2>)
	)
)]
#[get("/igdb/age-rating-content-descriptions-v2")]
pub async fn get_igdb_age_rating_content_descriptions_v2_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response =
		igdb_route_mutli_id_helper::<AgeRatingContentDescriptionV2>(query.into_inner().ids, |id| {
			tokio::spawn({
				let client = igdb_client.clone();
				let mut redis_conn = redis_conn.clone();
				async move {
					get_age_rating_content_description_v2_by_id_cached(
						client.as_ref(),
						&mut redis_conn,
						id,
					)
					.await
				}
			})
		})
		.await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for an Age Rating Content Description Type by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about an Age Rating Content Description Type", body = AgeRatingContentDescriptionType),
		(status = 404, description = "Age Rating Content Description Type not found")
	)
)]
#[get("/igdb/age-rating-content-description-type")]
pub async fn get_igdb_age_rating_content_description_type_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_age_rating_content_description_type_by_id_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().id,
	)
	.await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for Age Rating Content Description Types by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about Age Rating Content Description Types", body = Vec<AgeRatingContentDescriptionType>)
	)
)]
#[get("/igdb/age-rating-content-description-types")]
pub async fn get_igdb_age_rating_content_description_types_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<AgeRatingContentDescriptionType>(
		query.into_inner().ids,
		|id| {
			tokio::spawn({
				let client = igdb_client.clone();
				let mut redis_conn = redis_conn.clone();
				async move {
					get_age_rating_content_description_type_by_id_cached(
						client.as_ref(),
						&mut redis_conn,
						id,
					)
					.await
				}
			})
		},
	)
	.await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for an Age Rating Organization by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about an Age Rating Organization", body = AgeRatingOrganization),
		(status = 404, description = "Age Rating Organization not found")
	)
)]
#[get("/igdb/age-rating-organization")]
pub async fn get_igdb_age_rating_organization_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_age_rating_organization_by_id_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().id,
	)
	.await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for Age Rating Organizations by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about Age Rating Organizations", body = Vec<AgeRatingOrganization>)
	)
)]
#[get("/igdb/age-rating-organizations")]
pub async fn get_igdb_age_rating_organizations_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response =
		igdb_route_mutli_id_helper::<AgeRatingOrganization>(query.into_inner().ids, |id| {
			tokio::spawn({
				let client = igdb_client.clone();
				let mut redis_conn = redis_conn.clone();
				async move {
					get_age_rating_organization_by_id_cached(client.as_ref(), &mut redis_conn, id)
						.await
				}
			})
		})
		.await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Company Status by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a Company Status", body = CompanyStatus),
		(status = 404, description = "Company Status not found")
	)
)]
#[get("/igdb/company-status")]
pub async fn get_igdb_company_status_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_company_status_by_id_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().id,
	)
	.await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for Company Statuses by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about Company Statuses", body = Vec<CompanyStatus>)
	)
)]
#[get("/igdb/company-statuses")]
pub async fn get_igdb_company_statuses_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<CompanyStatus>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_company_status_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Date Format by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a Date Format", body = DateFormat),
		(status = 404, description = "Date Format not found")
	)
)]
#[get("/igdb/date-format")]
pub async fn get_igdb_date_format_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_date_format_by_id_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().id,
	)
	.await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for Date Formats by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about Date Formats", body = Vec<DateFormat>)
	)
)]
#[get("/igdb/date-formats")]
pub async fn get_igdb_date_formats_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<DateFormat>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_date_format_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for an External Game Source by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about an External Game Source", body = ExternalGameSource),
		(status = 404, description = "External Game Source not found")
	)
)]
#[get("/igdb/external-game-source")]
pub async fn get_igdb_external_game_source_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_external_game_source_by_id_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().id,
	)
	.await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for External Game Sources by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about External Game Sources", body = Vec<ExternalGameSource>)
	)
)]
#[get("/igdb/external-game-sources")]
pub async fn get_igdb_external_game_sources_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<ExternalGameSource>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move {
				get_external_game_source_by_id_cached(client.as_ref(), &mut redis_conn, id).await
			}
		})
	})
	.await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Game Release Format by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a Game Release Format", body = GameReleaseFormat),
		(status = 404, description = "Game Release Format not found")
	)
)]
#[get("/igdb/game-release-format")]
pub async fn get_igdb_game_release_format_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_game_release_format_by_id_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().id,
	)
	.await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for Game Release Formats by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about Game Release Formats", body = Vec<GameReleaseFormat>)
	)
)]
#[get("/igdb/game-release-formats")]
pub async fn get_igdb_game_release_formats_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<GameReleaseFormat>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move {
				get_game_release_format_by_id_cached(client.as_ref(), &mut redis_conn, id).await
			}
		})
	})
	.await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Game Status by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a Game Status", body = GameStatus),
		(status = 404, description = "Game Status not found")
	)
)]
#[get("/igdb/game-status")]
pub async fn get_igdb_game_status_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_game_status_by_id_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().id,
	)
	.await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for Game Statuses by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about Game Statuses", body = Vec<GameStatus>)
	)
)]
#[get("/igdb/game-statuses")]
pub async fn get_igdb_game_statuses_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<GameStatus>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_game_status_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Game Type by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a Game Type", body = GameType),
		(status = 404, description = "Game Type not found")
	)
)]
#[get("/igdb/game-type")]
pub async fn get_igdb_game_type_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_game_type_by_id_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().id,
	)
	.await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for Game Types by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about Game Types", body = Vec<GameType>)
	)
)]
#[get("/igdb/game-types")]
pub async fn get_igdb_game_types_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<GameType>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_game_type_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Platform Type by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a Platform Type", body = PlatformType),
		(status = 404, description = "Platform Type not found")
	)
)]
#[get("/igdb/platform-type")]
pub async fn get_igdb_platform_type_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_platform_type_by_id_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().id,
	)
	.await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for Platform Types by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about Platform Types", body = Vec<PlatformType>)
	)
)]
#[get("/igdb/platform-types")]
pub async fn get_igdb_platform_types_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<PlatformType>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_platform_type_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Release Date Region by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a Release Date Region", body = ReleaseDateRegion),
		(status = 404, description = "Release Date Region not found")
	)
)]
#[get("/igdb/release-date-region")]
pub async fn get_igdb_release_date_region_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_release_date_region_by_id_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().id,
	)
	.await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for Release Date Regions by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about Release Date Regions", body = Vec<ReleaseDateRegion>)
	)
)]
#[get("/igdb/release-date-regions")]
pub async fn get_igdb_release_date_regions_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<ReleaseDateRegion>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move {
				get_release_date_region_by_id_cached(client.as_ref(), &mut redis_conn, id).await
			}
		})
	})
	.await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Website Type by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a Website Type", body = WebsiteType),
		(status = 404, description = "Website Type not found")
	)
)]
#[get("/igdb/website-type")]
pub async fn get_igdb_website_type_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_website_type_by_id_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().id,
	)
	.await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for Website Types by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about Website Types", body = Vec<WebsiteType>)
	)
)]
#[get("/igdb/website-types")]
pub async fn get_igdb_website_types_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<WebsiteType>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_website_type_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for an Artwork Type by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about an Artwork Type", body = ArtworkType),
		(status = 404, description = "Artwork Type not found")
	)
)]
#[get("/igdb/artwork-type")]
pub async fn get_igdb_artwork_type_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_artwork_type_by_id_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().id,
	)
	.await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for Artwork Types by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about Artwork Types", body = Vec<ArtworkType>)
	)
)]
#[get("/igdb/artwork-types")]
pub async fn get_igdb_artwork_types_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<ArtworkType>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_artwork_type_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Character Mug Shot by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a Character Mug Shot", body = CharacterMugShot),
		(status = 404, description = "Character Mug Shot not found")
	)
)]
#[get("/igdb/character-mug-shot")]
pub async fn get_igdb_character_mug_shot_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_character_mug_shot_by_id_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().id,
	)
	.await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for Character Mug Shots by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about Character Mug Shots", body = Vec<CharacterMugShot>)
	)
)]
#[get("/igdb/character-mug-shots")]
pub async fn get_igdb_character_mug_shots_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<CharacterMugShot>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move {
				get_character_mug_shot_by_id_cached(client.as_ref(), &mut redis_conn, id).await
			}
		})
	})
	.await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Company Size by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a Company Size", body = CompanySize),
		(status = 404, description = "Company Size not found")
	)
)]
#[get("/igdb/company-size")]
pub async fn get_igdb_company_size_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_company_size_by_id_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().id,
	)
	.await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for Company Sizes by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about Company Sizes", body = Vec<CompanySize>)
	)
)]
#[get("/igdb/company-sizes")]
pub async fn get_igdb_company_sizes_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<CompanySize>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_company_size_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Company Type by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a Company Type", body = CompanyType),
		(status = 404, description = "Company Type not found")
	)
)]
#[get("/igdb/company-type")]
pub async fn get_igdb_company_type_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_company_type_by_id_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().id,
	)
	.await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for Company Types by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about Company Types", body = Vec<CompanyType>)
	)
)]
#[get("/igdb/company-types")]
pub async fn get_igdb_company_types_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<CompanyType>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_company_type_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Company Type History by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a Company Type History", body = CompanyTypeHistory),
		(status = 404, description = "Company Type History not found")
	)
)]
#[get("/igdb/company-type-history")]
pub async fn get_igdb_company_type_history_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_company_type_history_by_id_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().id,
	)
	.await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for Company Type Histories by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about Company Type Histories", body = Vec<CompanyTypeHistory>)
	)
)]
#[get("/igdb/company-type-histories")]
pub async fn get_igdb_company_type_histories_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<CompanyTypeHistory>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move {
				get_company_type_history_by_id_cached(client.as_ref(), &mut redis_conn, id).await
			}
		})
	})
	.await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for an Entity Type by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about an Entity Type", body = EntityType),
		(status = 404, description = "Entity Type not found")
	)
)]
#[get("/igdb/entity-type")]
pub async fn get_igdb_entity_type_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_entity_type_by_id_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().id,
	)
	.await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for Entity Types by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about Entity Types", body = Vec<EntityType>)
	)
)]
#[get("/igdb/entity-types")]
pub async fn get_igdb_entity_types_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<EntityType>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_entity_type_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Game Time To Beat by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a Game Time To Beat", body = GameTimeToBeat),
		(status = 404, description = "Game Time To Beat not found")
	)
)]
#[get("/igdb/game-time-to-beat")]
pub async fn get_igdb_game_time_to_beat_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_game_time_to_beat_by_id_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().id,
	)
	.await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for Game Time To Beats by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about Game Time To Beats", body = Vec<GameTimeToBeat>)
	)
)]
#[get("/igdb/game-time-to-beats")]
pub async fn get_igdb_game_time_to_beats_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<GameTimeToBeat>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_game_time_to_beat_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Report by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a Report", body = Report),
		(status = 404, description = "Report not found")
	)
)]
#[get("/igdb/report")]
pub async fn get_igdb_report_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_report_by_id_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().id,
	)
	.await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for Reports by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about Reports", body = Vec<Report>)
	)
)]
#[get("/igdb/reports")]
pub async fn get_igdb_reports_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<Report>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_report_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Report Type by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a Report Type", body = ReportType),
		(status = 404, description = "Report Type not found")
	)
)]
#[get("/igdb/report-type")]
pub async fn get_igdb_report_type_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_report_type_by_id_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query.into_inner().id,
	)
	.await?;

	if response.is_none() {
		return Ok(HttpResponse::NotFound().finish());
	}

	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for Report Types by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about Report Types", body = Vec<ReportType>)
	)
)]
#[get("/igdb/report-types")]
pub async fn get_igdb_report_types_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<ReportType>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_report_type_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;

	Ok(HttpResponse::Ok().json(response))
}
