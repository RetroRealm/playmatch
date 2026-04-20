use crate::error;
use crate::model::igdb::{IdQuery, IdsQuery, SearchQuery, SlugIdQuery, validate_search_literal};
use crate::util::igdb_route_mutli_id_helper;
use actix_web::web::Data;
use actix_web::{HttpResponse, Responder, get};
use actix_web_lab::extract::Query;
use service::providers::igdb::IgdbClient;
use service::providers::igdb::cache::{
	get_age_rating_by_id_cached, get_age_rating_category_by_id_cached,
	get_age_rating_content_description_type_by_id_cached,
	get_age_rating_content_description_v2_by_id_cached, get_age_rating_organization_by_id_cached,
	get_alternative_name_by_id_cached, get_artwork_by_id_cached, get_artwork_type_by_id_cached,
	get_character_by_id_cached, get_character_gender_by_id_cached,
	get_character_mug_shot_by_id_cached, get_character_species_by_id_cached,
	get_collection_by_id_cached, get_collection_membership_by_id_cached,
	get_collection_membership_type_by_id_cached, get_collection_relation_by_id_cached,
	get_collection_relation_type_by_id_cached, get_collection_type_by_id_cached,
	get_company_by_id_cached, get_company_logo_by_id_cached, get_company_size_by_id_cached,
	get_company_status_by_id_cached, get_company_type_by_id_cached,
	get_company_type_history_by_id_cached, get_company_website_by_id_cached,
	get_cover_by_id_cached, get_date_format_by_id_cached, get_entity_type_by_id_cached,
	get_event_by_id_cached, get_event_logo_by_id_cached, get_event_network_by_id_cached,
	get_external_game_by_id_cached, get_external_game_source_by_id_cached,
	get_franchise_by_id_cached, get_game_by_id_cached, get_game_by_slug_cached,
	get_game_engine_by_id_cached, get_game_engine_logo_by_id_cached,
	get_game_localization_by_id_cached, get_game_mode_by_id_cached,
	get_game_release_format_by_id_cached, get_game_status_by_id_cached,
	get_game_time_to_beat_by_id_cached, get_game_type_by_id_cached, get_game_version_by_id_cached,
	get_game_version_feature_by_id_cached, get_game_version_feature_value_by_id_cached,
	get_game_video_by_id_cached, get_genre_by_id_cached, get_involved_company_by_id_cached,
	get_keyword_by_id_cached, get_language_by_id_cached, get_language_support_by_id_cached,
	get_language_support_type_by_id_cached, get_multiplayer_mode_by_id_cached,
	get_network_type_by_id_cached, get_platform_by_id_cached, get_platform_family_by_id_cached,
	get_platform_logo_by_id_cached, get_platform_type_by_id_cached,
	get_platform_version_by_id_cached, get_platform_version_company_by_id_cached,
	get_platform_version_release_date_by_id_cached, get_platform_website_by_id_cached,
	get_player_perspective_by_id_cached, get_popularity_primitive_by_id_cached,
	get_popularity_type_by_id_cached, get_region_by_id_cached, get_release_date_by_id_cached,
	get_release_date_region_by_id_cached, get_release_date_status_by_id_cached,
	get_report_by_id_cached, get_report_type_by_id_cached, get_screenshot_by_id_cached,
	get_theme_by_id_cached, get_website_by_id_cached, get_website_type_by_id_cached,
	search_game_by_name_cached,
};
use service::providers::igdb::model::{
	AgeRating, AgeRatingCategory, AgeRatingContentDescriptionType, AgeRatingContentDescriptionV2,
	AgeRatingOrganization, AlternativeName, Artwork, ArtworkType, Character, CharacterGender,
	CharacterMugShot, CharacterSpecies, Collection, CollectionMembership, CollectionMembershipType,
	CollectionRelation, CollectionRelationType, CollectionType, Company, CompanyLogo, CompanySize,
	CompanyStatus, CompanyType, CompanyTypeHistory, CompanyWebsite, Cover, DateFormat, EntityType,
	Event, EventLogo, EventNetwork, ExternalGame, ExternalGameSource, Franchise, Game, GameEngine,
	GameEngineLogo, GameLocalization, GameMode, GameReleaseFormat, GameStatus, GameTimeToBeat,
	GameType, GameVersion, GameVersionFeature, GameVersionFeatureValue, GameVideo, Genre,
	InvolvedCompany, Keyword, Language, LanguageSupport, LanguageSupportType, MultiplayerMode,
	NetworkType, Platform, PlatformFamily, PlatformLogo, PlatformType, PlatformVersion,
	PlatformVersionCompany, PlatformVersionReleaseDate, PlatformWebsite, PlayerPerspective,
	PopularityPrimitive, PopularityType, Region, ReleaseDate, ReleaseDateRegion, ReleaseDateStatus,
	Report, ReportType, Screenshot, Theme, Website, WebsiteType,
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
		if let Err(resp) = validate_search_literal(&slug) {
			return Ok(resp);
		}
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
	let query = query.into_inner().query;
	if let Err(resp) = validate_search_literal(&query) {
		return Ok(resp);
	}

	let response = search_game_by_name_cached(
		igdb_client.as_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
		query,
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

/// Queries the IGDB API for a Character by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a character", body = Character),
		(status = 404, description = "Character not found")
	)
)]
#[get("/igdb/character")]
pub async fn get_igdb_character_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_character_by_id_cached(
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

/// Queries the IGDB API for Characters by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about characters", body = Vec<Character>)
	)
)]
#[get("/igdb/characters")]
pub async fn get_igdb_characters_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<Character>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_character_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Character Gender by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a character gender", body = CharacterGender),
		(status = 404, description = "Character gender not found")
	)
)]
#[get("/igdb/character-gender")]
pub async fn get_igdb_character_gender_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_character_gender_by_id_cached(
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

/// Queries the IGDB API for Character Genders by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about character genders", body = Vec<CharacterGender>)
	)
)]
#[get("/igdb/character-genders")]
pub async fn get_igdb_character_genders_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<CharacterGender>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_character_gender_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Character Species by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a character species", body = CharacterSpecies),
		(status = 404, description = "Character species not found")
	)
)]
#[get("/igdb/character-species")]
pub async fn get_igdb_character_species_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_character_species_by_id_cached(
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

/// Queries the IGDB API for multiple Character Species by Ids. "Species" is
/// both singular and plural; the path carries a `-list` suffix to avoid
/// colliding with the single-id route.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about character species", body = Vec<CharacterSpecies>)
	)
)]
#[get("/igdb/character-species-list")]
pub async fn get_igdb_character_species_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<CharacterSpecies>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_character_species_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Collection Membership by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a collection membership", body = CollectionMembership),
		(status = 404, description = "Collection membership not found")
	)
)]
#[get("/igdb/collection-membership")]
pub async fn get_igdb_collection_membership_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_collection_membership_by_id_cached(
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

/// Queries the IGDB API for Collection Memberships by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about collection memberships", body = Vec<CollectionMembership>)
	)
)]
#[get("/igdb/collection-memberships")]
pub async fn get_igdb_collection_memberships_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response =
		igdb_route_mutli_id_helper::<CollectionMembership>(query.into_inner().ids, |id| {
			tokio::spawn({
				let client = igdb_client.clone();
				let mut redis_conn = redis_conn.clone();
				async move {
					get_collection_membership_by_id_cached(client.as_ref(), &mut redis_conn, id)
						.await
				}
			})
		})
		.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Collection Membership Type by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a collection membership type", body = CollectionMembershipType),
		(status = 404, description = "Collection membership type not found")
	)
)]
#[get("/igdb/collection-membership-type")]
pub async fn get_igdb_collection_membership_type_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_collection_membership_type_by_id_cached(
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

/// Queries the IGDB API for Collection Membership Types by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about collection membership types", body = Vec<CollectionMembershipType>)
	)
)]
#[get("/igdb/collection-membership-types")]
pub async fn get_igdb_collection_membership_types_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response =
		igdb_route_mutli_id_helper::<CollectionMembershipType>(query.into_inner().ids, |id| {
			tokio::spawn({
				let client = igdb_client.clone();
				let mut redis_conn = redis_conn.clone();
				async move {
					get_collection_membership_type_by_id_cached(
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

/// Queries the IGDB API for a Collection Relation by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a collection relation", body = CollectionRelation),
		(status = 404, description = "Collection relation not found")
	)
)]
#[get("/igdb/collection-relation")]
pub async fn get_igdb_collection_relation_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_collection_relation_by_id_cached(
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

/// Queries the IGDB API for Collection Relations by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about collection relations", body = Vec<CollectionRelation>)
	)
)]
#[get("/igdb/collection-relations")]
pub async fn get_igdb_collection_relations_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<CollectionRelation>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move {
				get_collection_relation_by_id_cached(client.as_ref(), &mut redis_conn, id).await
			}
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Collection Relation Type by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a collection relation type", body = CollectionRelationType),
		(status = 404, description = "Collection relation type not found")
	)
)]
#[get("/igdb/collection-relation-type")]
pub async fn get_igdb_collection_relation_type_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_collection_relation_type_by_id_cached(
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

/// Queries the IGDB API for Collection Relation Types by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about collection relation types", body = Vec<CollectionRelationType>)
	)
)]
#[get("/igdb/collection-relation-types")]
pub async fn get_igdb_collection_relation_types_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response =
		igdb_route_mutli_id_helper::<CollectionRelationType>(query.into_inner().ids, |id| {
			tokio::spawn({
				let client = igdb_client.clone();
				let mut redis_conn = redis_conn.clone();
				async move {
					get_collection_relation_type_by_id_cached(client.as_ref(), &mut redis_conn, id)
						.await
				}
			})
		})
		.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Collection Type by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a collection type", body = CollectionType),
		(status = 404, description = "Collection type not found")
	)
)]
#[get("/igdb/collection-type")]
pub async fn get_igdb_collection_type_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_collection_type_by_id_cached(
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

/// Queries the IGDB API for Collection Types by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about collection types", body = Vec<CollectionType>)
	)
)]
#[get("/igdb/collection-types")]
pub async fn get_igdb_collection_types_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<CollectionType>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_collection_type_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Company by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a company", body = Company),
		(status = 404, description = "Company not found")
	)
)]
#[get("/igdb/company")]
pub async fn get_igdb_company_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_company_by_id_cached(
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

/// Queries the IGDB API for Companies by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about companies", body = Vec<Company>)
	)
)]
#[get("/igdb/companies")]
pub async fn get_igdb_companies_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<Company>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_company_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Company Logo by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a company logo", body = CompanyLogo),
		(status = 404, description = "Company logo not found")
	)
)]
#[get("/igdb/company-logo")]
pub async fn get_igdb_company_logo_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_company_logo_by_id_cached(
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

/// Queries the IGDB API for Company Logos by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about company logos", body = Vec<CompanyLogo>)
	)
)]
#[get("/igdb/company-logos")]
pub async fn get_igdb_company_logos_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<CompanyLogo>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_company_logo_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Company Website by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a company website", body = CompanyWebsite),
		(status = 404, description = "Company website not found")
	)
)]
#[get("/igdb/company-website")]
pub async fn get_igdb_company_website_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_company_website_by_id_cached(
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

/// Queries the IGDB API for Company Websites by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about company websites", body = Vec<CompanyWebsite>)
	)
)]
#[get("/igdb/company-websites")]
pub async fn get_igdb_company_websites_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<CompanyWebsite>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_company_website_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for an Event by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about an event", body = Event),
		(status = 404, description = "Event not found")
	)
)]
#[get("/igdb/event")]
pub async fn get_igdb_event_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_event_by_id_cached(
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

/// Queries the IGDB API for Events by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about events", body = Vec<Event>)
	)
)]
#[get("/igdb/events")]
pub async fn get_igdb_events_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<Event>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_event_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for an Event Logo by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about an event logo", body = EventLogo),
		(status = 404, description = "Event logo not found")
	)
)]
#[get("/igdb/event-logo")]
pub async fn get_igdb_event_logo_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_event_logo_by_id_cached(
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

/// Queries the IGDB API for Event Logos by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about event logos", body = Vec<EventLogo>)
	)
)]
#[get("/igdb/event-logos")]
pub async fn get_igdb_event_logos_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<EventLogo>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_event_logo_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for an Event Network by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about an event network", body = EventNetwork),
		(status = 404, description = "Event network not found")
	)
)]
#[get("/igdb/event-network")]
pub async fn get_igdb_event_network_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_event_network_by_id_cached(
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

/// Queries the IGDB API for Event Networks by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about event networks", body = Vec<EventNetwork>)
	)
)]
#[get("/igdb/event-networks")]
pub async fn get_igdb_event_networks_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<EventNetwork>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_event_network_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Game Engine by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a game engine", body = GameEngine),
		(status = 404, description = "Game engine not found")
	)
)]
#[get("/igdb/game-engine")]
pub async fn get_igdb_game_engine_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_game_engine_by_id_cached(
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

/// Queries the IGDB API for Game Engines by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about game engines", body = Vec<GameEngine>)
	)
)]
#[get("/igdb/game-engines")]
pub async fn get_igdb_game_engines_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<GameEngine>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_game_engine_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Game Engine Logo by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a game engine logo", body = GameEngineLogo),
		(status = 404, description = "Game engine logo not found")
	)
)]
#[get("/igdb/game-engine-logo")]
pub async fn get_igdb_game_engine_logo_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_game_engine_logo_by_id_cached(
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

/// Queries the IGDB API for Game Engine Logos by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about game engine logos", body = Vec<GameEngineLogo>)
	)
)]
#[get("/igdb/game-engine-logos")]
pub async fn get_igdb_game_engine_logos_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<GameEngineLogo>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_game_engine_logo_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Game Localization by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a game localization", body = GameLocalization),
		(status = 404, description = "Game localization not found")
	)
)]
#[get("/igdb/game-localization")]
pub async fn get_igdb_game_localization_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_game_localization_by_id_cached(
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

/// Queries the IGDB API for Game Localizations by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about game localizations", body = Vec<GameLocalization>)
	)
)]
#[get("/igdb/game-localizations")]
pub async fn get_igdb_game_localizations_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<GameLocalization>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_game_localization_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Game Mode by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a game mode", body = GameMode),
		(status = 404, description = "Game mode not found")
	)
)]
#[get("/igdb/game-mode")]
pub async fn get_igdb_game_mode_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_game_mode_by_id_cached(
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

/// Queries the IGDB API for Game Modes by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about game modes", body = Vec<GameMode>)
	)
)]
#[get("/igdb/game-modes")]
pub async fn get_igdb_game_modes_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<GameMode>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_game_mode_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Game Version by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a game version", body = GameVersion),
		(status = 404, description = "Game version not found")
	)
)]
#[get("/igdb/game-version")]
pub async fn get_igdb_game_version_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_game_version_by_id_cached(
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

/// Queries the IGDB API for Game Versions by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about game versions", body = Vec<GameVersion>)
	)
)]
#[get("/igdb/game-versions")]
pub async fn get_igdb_game_versions_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<GameVersion>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_game_version_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Game Version Feature by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a game version feature", body = GameVersionFeature),
		(status = 404, description = "Game version feature not found")
	)
)]
#[get("/igdb/game-version-feature")]
pub async fn get_igdb_game_version_feature_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_game_version_feature_by_id_cached(
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

/// Queries the IGDB API for Game Version Features by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about game version features", body = Vec<GameVersionFeature>)
	)
)]
#[get("/igdb/game-version-features")]
pub async fn get_igdb_game_version_features_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<GameVersionFeature>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move {
				get_game_version_feature_by_id_cached(client.as_ref(), &mut redis_conn, id).await
			}
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Game Version Feature Value by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a game version feature value", body = GameVersionFeatureValue),
		(status = 404, description = "Game version feature value not found")
	)
)]
#[get("/igdb/game-version-feature-value")]
pub async fn get_igdb_game_version_feature_value_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_game_version_feature_value_by_id_cached(
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

/// Queries the IGDB API for Game Version Feature Values by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about game version feature values", body = Vec<GameVersionFeatureValue>)
	)
)]
#[get("/igdb/game-version-feature-values")]
pub async fn get_igdb_game_version_feature_values_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response =
		igdb_route_mutli_id_helper::<GameVersionFeatureValue>(query.into_inner().ids, |id| {
			tokio::spawn({
				let client = igdb_client.clone();
				let mut redis_conn = redis_conn.clone();
				async move {
					get_game_version_feature_value_by_id_cached(
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

/// Queries the IGDB API for a Game Video by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a game video", body = GameVideo),
		(status = 404, description = "Game video not found")
	)
)]
#[get("/igdb/game-video")]
pub async fn get_igdb_game_video_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_game_video_by_id_cached(
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

/// Queries the IGDB API for Game Videos by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about game videos", body = Vec<GameVideo>)
	)
)]
#[get("/igdb/game-videos")]
pub async fn get_igdb_game_videos_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<GameVideo>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_game_video_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for an Involved Company by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about an involved company", body = InvolvedCompany),
		(status = 404, description = "Involved company not found")
	)
)]
#[get("/igdb/involved-company")]
pub async fn get_igdb_involved_company_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_involved_company_by_id_cached(
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

/// Queries the IGDB API for Involved Companies by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about involved companies", body = Vec<InvolvedCompany>)
	)
)]
#[get("/igdb/involved-companies")]
pub async fn get_igdb_involved_companies_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<InvolvedCompany>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_involved_company_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Keyword by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a keyword", body = Keyword),
		(status = 404, description = "Keyword not found")
	)
)]
#[get("/igdb/keyword")]
pub async fn get_igdb_keyword_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_keyword_by_id_cached(
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

/// Queries the IGDB API for Keywords by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about keywords", body = Vec<Keyword>)
	)
)]
#[get("/igdb/keywords")]
pub async fn get_igdb_keywords_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<Keyword>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_keyword_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Language by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a language", body = Language),
		(status = 404, description = "Language not found")
	)
)]
#[get("/igdb/language")]
pub async fn get_igdb_language_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_language_by_id_cached(
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

/// Queries the IGDB API for Languages by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about languages", body = Vec<Language>)
	)
)]
#[get("/igdb/languages")]
pub async fn get_igdb_languages_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<Language>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_language_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Language Support by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a language support", body = LanguageSupport),
		(status = 404, description = "Language support not found")
	)
)]
#[get("/igdb/language-support")]
pub async fn get_igdb_language_support_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_language_support_by_id_cached(
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

/// Queries the IGDB API for Language Supports by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about language supports", body = Vec<LanguageSupport>)
	)
)]
#[get("/igdb/language-supports")]
pub async fn get_igdb_language_supports_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<LanguageSupport>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_language_support_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Language Support Type by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a language support type", body = LanguageSupportType),
		(status = 404, description = "Language support type not found")
	)
)]
#[get("/igdb/language-support-type")]
pub async fn get_igdb_language_support_type_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_language_support_type_by_id_cached(
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

/// Queries the IGDB API for Language Support Types by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about language support types", body = Vec<LanguageSupportType>)
	)
)]
#[get("/igdb/language-support-types")]
pub async fn get_igdb_language_support_types_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response =
		igdb_route_mutli_id_helper::<LanguageSupportType>(query.into_inner().ids, |id| {
			tokio::spawn({
				let client = igdb_client.clone();
				let mut redis_conn = redis_conn.clone();
				async move {
					get_language_support_type_by_id_cached(client.as_ref(), &mut redis_conn, id)
						.await
				}
			})
		})
		.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Multiplayer Mode by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a multiplayer mode", body = MultiplayerMode),
		(status = 404, description = "Multiplayer mode not found")
	)
)]
#[get("/igdb/multiplayer-mode")]
pub async fn get_igdb_multiplayer_mode_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_multiplayer_mode_by_id_cached(
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

/// Queries the IGDB API for Multiplayer Modes by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about multiplayer modes", body = Vec<MultiplayerMode>)
	)
)]
#[get("/igdb/multiplayer-modes")]
pub async fn get_igdb_multiplayer_modes_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<MultiplayerMode>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_multiplayer_mode_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Network Type by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a network type", body = NetworkType),
		(status = 404, description = "Network type not found")
	)
)]
#[get("/igdb/network-type")]
pub async fn get_igdb_network_type_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_network_type_by_id_cached(
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

/// Queries the IGDB API for Network Types by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about network types", body = Vec<NetworkType>)
	)
)]
#[get("/igdb/network-types")]
pub async fn get_igdb_network_types_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<NetworkType>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_network_type_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Platform by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a platform", body = Platform),
		(status = 404, description = "Platform not found")
	)
)]
#[get("/igdb/platform")]
pub async fn get_igdb_platform_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_platform_by_id_cached(
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

/// Queries the IGDB API for Platforms by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about platforms", body = Vec<Platform>)
	)
)]
#[get("/igdb/platforms")]
pub async fn get_igdb_platforms_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<Platform>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_platform_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Platform Family by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a platform family", body = PlatformFamily),
		(status = 404, description = "Platform family not found")
	)
)]
#[get("/igdb/platform-family")]
pub async fn get_igdb_platform_family_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_platform_family_by_id_cached(
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

/// Queries the IGDB API for Platform Families by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about platform families", body = Vec<PlatformFamily>)
	)
)]
#[get("/igdb/platform-families")]
pub async fn get_igdb_platform_families_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<PlatformFamily>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_platform_family_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Platform Logo by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a platform logo", body = PlatformLogo),
		(status = 404, description = "Platform logo not found")
	)
)]
#[get("/igdb/platform-logo")]
pub async fn get_igdb_platform_logo_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_platform_logo_by_id_cached(
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

/// Queries the IGDB API for Platform Logos by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about platform logos", body = Vec<PlatformLogo>)
	)
)]
#[get("/igdb/platform-logos")]
pub async fn get_igdb_platform_logos_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<PlatformLogo>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_platform_logo_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Platform Version by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a platform version", body = PlatformVersion),
		(status = 404, description = "Platform version not found")
	)
)]
#[get("/igdb/platform-version")]
pub async fn get_igdb_platform_version_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_platform_version_by_id_cached(
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

/// Queries the IGDB API for Platform Versions by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about platform versions", body = Vec<PlatformVersion>)
	)
)]
#[get("/igdb/platform-versions")]
pub async fn get_igdb_platform_versions_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<PlatformVersion>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_platform_version_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Platform Version Company by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a platform version company", body = PlatformVersionCompany),
		(status = 404, description = "Platform version company not found")
	)
)]
#[get("/igdb/platform-version-company")]
pub async fn get_igdb_platform_version_company_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_platform_version_company_by_id_cached(
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

/// Queries the IGDB API for Platform Version Companies by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about platform version companies", body = Vec<PlatformVersionCompany>)
	)
)]
#[get("/igdb/platform-version-companies")]
pub async fn get_igdb_platform_version_companies_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response =
		igdb_route_mutli_id_helper::<PlatformVersionCompany>(query.into_inner().ids, |id| {
			tokio::spawn({
				let client = igdb_client.clone();
				let mut redis_conn = redis_conn.clone();
				async move {
					get_platform_version_company_by_id_cached(client.as_ref(), &mut redis_conn, id)
						.await
				}
			})
		})
		.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Platform Version Release Date by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a platform version release date", body = PlatformVersionReleaseDate),
		(status = 404, description = "Platform version release date not found")
	)
)]
#[get("/igdb/platform-version-release-date")]
pub async fn get_igdb_platform_version_release_date_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_platform_version_release_date_by_id_cached(
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

/// Queries the IGDB API for Platform Version Release Dates by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about platform version release dates", body = Vec<PlatformVersionReleaseDate>)
	)
)]
#[get("/igdb/platform-version-release-dates")]
pub async fn get_igdb_platform_version_release_dates_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response =
		igdb_route_mutli_id_helper::<PlatformVersionReleaseDate>(query.into_inner().ids, |id| {
			tokio::spawn({
				let client = igdb_client.clone();
				let mut redis_conn = redis_conn.clone();
				async move {
					get_platform_version_release_date_by_id_cached(
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

/// Queries the IGDB API for a Platform Website by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a platform website", body = PlatformWebsite),
		(status = 404, description = "Platform website not found")
	)
)]
#[get("/igdb/platform-website")]
pub async fn get_igdb_platform_website_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_platform_website_by_id_cached(
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

/// Queries the IGDB API for Platform Websites by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about platform websites", body = Vec<PlatformWebsite>)
	)
)]
#[get("/igdb/platform-websites")]
pub async fn get_igdb_platform_websites_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<PlatformWebsite>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_platform_website_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Player Perspective by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a player perspective", body = PlayerPerspective),
		(status = 404, description = "Player perspective not found")
	)
)]
#[get("/igdb/player-perspective")]
pub async fn get_igdb_player_perspective_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_player_perspective_by_id_cached(
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

/// Queries the IGDB API for Player Perspectives by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about player perspectives", body = Vec<PlayerPerspective>)
	)
)]
#[get("/igdb/player-perspectives")]
pub async fn get_igdb_player_perspectives_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<PlayerPerspective>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move {
				get_player_perspective_by_id_cached(client.as_ref(), &mut redis_conn, id).await
			}
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Popularity Primitive by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a popularity primitive", body = PopularityPrimitive),
		(status = 404, description = "Popularity primitive not found")
	)
)]
#[get("/igdb/popularity-primitive")]
pub async fn get_igdb_popularity_primitive_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_popularity_primitive_by_id_cached(
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

/// Queries the IGDB API for Popularity Primitives by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about popularity primitives", body = Vec<PopularityPrimitive>)
	)
)]
#[get("/igdb/popularity-primitives")]
pub async fn get_igdb_popularity_primitives_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response =
		igdb_route_mutli_id_helper::<PopularityPrimitive>(query.into_inner().ids, |id| {
			tokio::spawn({
				let client = igdb_client.clone();
				let mut redis_conn = redis_conn.clone();
				async move {
					get_popularity_primitive_by_id_cached(client.as_ref(), &mut redis_conn, id)
						.await
				}
			})
		})
		.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Popularity Type by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a popularity type", body = PopularityType),
		(status = 404, description = "Popularity type not found")
	)
)]
#[get("/igdb/popularity-type")]
pub async fn get_igdb_popularity_type_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_popularity_type_by_id_cached(
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

/// Queries the IGDB API for Popularity Types by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about popularity types", body = Vec<PopularityType>)
	)
)]
#[get("/igdb/popularity-types")]
pub async fn get_igdb_popularity_types_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<PopularityType>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_popularity_type_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Region by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a region", body = Region),
		(status = 404, description = "Region not found")
	)
)]
#[get("/igdb/region")]
pub async fn get_igdb_region_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_region_by_id_cached(
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

/// Queries the IGDB API for Regions by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about regions", body = Vec<Region>)
	)
)]
#[get("/igdb/regions")]
pub async fn get_igdb_regions_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<Region>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_region_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Release Date by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a release date", body = ReleaseDate),
		(status = 404, description = "Release date not found")
	)
)]
#[get("/igdb/release-date")]
pub async fn get_igdb_release_date_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_release_date_by_id_cached(
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

/// Queries the IGDB API for Release Dates by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about release dates", body = Vec<ReleaseDate>)
	)
)]
#[get("/igdb/release-dates")]
pub async fn get_igdb_release_dates_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<ReleaseDate>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_release_date_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Release Date Status by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a release date status", body = ReleaseDateStatus),
		(status = 404, description = "Release date status not found")
	)
)]
#[get("/igdb/release-date-status")]
pub async fn get_igdb_release_date_status_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_release_date_status_by_id_cached(
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

/// Queries the IGDB API for Release Date Statuses by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about release date statuses", body = Vec<ReleaseDateStatus>)
	)
)]
#[get("/igdb/release-date-statuses")]
pub async fn get_igdb_release_date_statuses_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<ReleaseDateStatus>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move {
				get_release_date_status_by_id_cached(client.as_ref(), &mut redis_conn, id).await
			}
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Screenshot by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a screenshot", body = Screenshot),
		(status = 404, description = "Screenshot not found")
	)
)]
#[get("/igdb/screenshot")]
pub async fn get_igdb_screenshot_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_screenshot_by_id_cached(
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

/// Queries the IGDB API for Screenshots by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about screenshots", body = Vec<Screenshot>)
	)
)]
#[get("/igdb/screenshots")]
pub async fn get_igdb_screenshots_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<Screenshot>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_screenshot_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Theme by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a theme", body = Theme),
		(status = 404, description = "Theme not found")
	)
)]
#[get("/igdb/theme")]
pub async fn get_igdb_theme_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_theme_by_id_cached(
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

/// Queries the IGDB API for Themes by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about themes", body = Vec<Theme>)
	)
)]
#[get("/igdb/themes")]
pub async fn get_igdb_themes_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<Theme>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_theme_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}

/// Queries the IGDB API for a Website by Id
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about a website", body = Website),
		(status = 404, description = "Website not found")
	)
)]
#[get("/igdb/website")]
pub async fn get_igdb_website_by_id(
	query: Query<IdQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let response = get_website_by_id_cached(
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

/// Queries the IGDB API for Websites by Ids
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "Returns IGDB metadata about websites", body = Vec<Website>)
	)
)]
#[get("/igdb/websites")]
pub async fn get_igdb_websites_by_ids(
	query: Query<IdsQuery>,
	redis_client: Data<redis::Client>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	let response = igdb_route_mutli_id_helper::<Website>(query.into_inner().ids, |id| {
		tokio::spawn({
			let client = igdb_client.clone();
			let mut redis_conn = redis_conn.clone();
			async move { get_website_by_id_cached(client.as_ref(), &mut redis_conn, id).await }
		})
	})
	.await?;
	Ok(HttpResponse::Ok().json(response))
}
