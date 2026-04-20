use crate::error;
use crate::model::igdb::{IdQuery, IdsQuery, SearchQuery, SlugIdQuery, validate_search_literal};
use crate::util::igdb_route_mutli_id_helper;
use actix_web::web::Data;
use actix_web::{HttpResponse, Responder, get};
use actix_web_lab::extract::Query;
use redis::aio::MultiplexedConnection;
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

macro_rules! igdb_id_route {
	($route:literal, $fn_name:ident, $cached_fn:ident, $model:ty) => {
		#[utoipa::path(
			get,
			context_path = "/api",
			tag = "IGDB",
			params(IdQuery),
			responses(
				(status = 200, description = "Returns IGDB metadata for the requested id", body = $model),
				(status = 404, description = "Not found")
			)
		)]
		#[get($route)]
		pub async fn $fn_name(
			query: Query<IdQuery>,
			redis_conn: Data<MultiplexedConnection>,
			igdb_client: Data<IgdbClient>,
		) -> error::Result<impl Responder> {
			let response = $cached_fn(
				igdb_client.as_ref(),
				&mut redis_conn.get_ref().clone(),
				query.into_inner().id,
			)
			.await?;

			if response.is_none() {
				return Ok(HttpResponse::NotFound().finish());
			}

			Ok(HttpResponse::Ok().json(response))
		}
	};
}

macro_rules! igdb_ids_route {
	($route:literal, $fn_name:ident, $cached_fn:ident, $model:ty) => {
		#[utoipa::path(
			get,
			context_path = "/api",
			tag = "IGDB",
			params(IdsQuery),
			responses(
				(status = 200, description = "Returns IGDB metadata for the requested ids", body = Vec<$model>)
			)
		)]
		#[get($route)]
		pub async fn $fn_name(
			query: Query<IdsQuery>,
			redis_conn: Data<MultiplexedConnection>,
			igdb_client: Data<IgdbClient>,
		) -> error::Result<impl Responder> {
			let redis_conn = redis_conn.get_ref().clone();

			let response = igdb_route_mutli_id_helper::<$model>(query.into_inner().ids, |id| {
				tokio::spawn({
					let client = igdb_client.clone();
					let mut redis_conn = redis_conn.clone();
					async move { $cached_fn(client.as_ref(), &mut redis_conn, id).await }
				})
			})
			.await?;

			Ok(HttpResponse::Ok().json(response))
		}
	};
}

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
	redis_conn: Data<MultiplexedConnection>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let query = query.into_inner();
	let mut redis_conn = redis_conn.get_ref().clone();

	let response = if let Some(id) = query.id {
		get_game_by_id_cached(igdb_client.as_ref(), &mut redis_conn, id).await?
	} else if let Some(slug) = query.slug {
		if let Err(resp) = validate_search_literal(&slug) {
			return Ok(resp);
		}
		get_game_by_slug_cached(igdb_client.as_ref(), &mut redis_conn, slug).await?
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
	redis_conn: Data<MultiplexedConnection>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let redis_conn = redis_conn.get_ref().clone();

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
	redis_conn: Data<MultiplexedConnection>,
	igdb_client: Data<IgdbClient>,
) -> error::Result<impl Responder> {
	let query = query.into_inner().query;
	if let Err(resp) = validate_search_literal(&query) {
		return Ok(resp);
	}

	let response = search_game_by_name_cached(
		igdb_client.as_ref(),
		&mut redis_conn.get_ref().clone(),
		query,
	)
	.await?;

	Ok(HttpResponse::Ok().json(response))
}

igdb_id_route!(
	"/igdb/age-rating",
	get_igdb_age_rating_by_id,
	get_age_rating_by_id_cached,
	AgeRating
);
igdb_ids_route!(
	"/igdb/age-ratings",
	get_igdb_age_ratings_by_ids,
	get_age_rating_by_id_cached,
	AgeRating
);

igdb_id_route!(
	"/igdb/alternative-name",
	get_igdb_alternative_name_by_id,
	get_alternative_name_by_id_cached,
	AlternativeName
);
igdb_ids_route!(
	"/igdb/alternative-names",
	get_igdb_alternative_names_by_ids,
	get_alternative_name_by_id_cached,
	AlternativeName
);

igdb_id_route!(
	"/igdb/artwork",
	get_igdb_artwork_by_id,
	get_artwork_by_id_cached,
	Artwork
);
igdb_ids_route!(
	"/igdb/artworks",
	get_igdb_artworks_by_ids,
	get_artwork_by_id_cached,
	Artwork
);

igdb_id_route!(
	"/igdb/collection",
	get_igdb_collection_by_id,
	get_collection_by_id_cached,
	Collection
);
igdb_ids_route!(
	"/igdb/collections",
	get_igdb_collections_by_ids,
	get_collection_by_id_cached,
	Collection
);

igdb_id_route!(
	"/igdb/cover",
	get_igdb_cover_by_id,
	get_cover_by_id_cached,
	Cover
);
igdb_ids_route!(
	"/igdb/covers",
	get_igdb_covers_by_ids,
	get_cover_by_id_cached,
	Cover
);

igdb_id_route!(
	"/igdb/external-game",
	get_igdb_external_game_by_id,
	get_external_game_by_id_cached,
	ExternalGame
);
igdb_ids_route!(
	"/igdb/external-games",
	get_igdb_external_games_by_ids,
	get_external_game_by_id_cached,
	ExternalGame
);

igdb_id_route!(
	"/igdb/franchise",
	get_igdb_franchise_by_id,
	get_franchise_by_id_cached,
	Franchise
);
igdb_ids_route!(
	"/igdb/franchises",
	get_igdb_franchises_by_ids,
	get_franchise_by_id_cached,
	Franchise
);

igdb_id_route!(
	"/igdb/genre",
	get_igdb_genre_by_id,
	get_genre_by_id_cached,
	Genre
);
igdb_ids_route!(
	"/igdb/genres",
	get_igdb_genres_by_ids,
	get_genre_by_id_cached,
	Genre
);

igdb_id_route!(
	"/igdb/age-rating-category",
	get_igdb_age_rating_category_by_id,
	get_age_rating_category_by_id_cached,
	AgeRatingCategory
);
igdb_ids_route!(
	"/igdb/age-rating-categories",
	get_igdb_age_rating_categories_by_ids,
	get_age_rating_category_by_id_cached,
	AgeRatingCategory
);

igdb_id_route!(
	"/igdb/age-rating-content-description-v2",
	get_igdb_age_rating_content_description_v2_by_id,
	get_age_rating_content_description_v2_by_id_cached,
	AgeRatingContentDescriptionV2
);
igdb_ids_route!(
	"/igdb/age-rating-content-descriptions-v2",
	get_igdb_age_rating_content_descriptions_v2_by_ids,
	get_age_rating_content_description_v2_by_id_cached,
	AgeRatingContentDescriptionV2
);

igdb_id_route!(
	"/igdb/age-rating-content-description-type",
	get_igdb_age_rating_content_description_type_by_id,
	get_age_rating_content_description_type_by_id_cached,
	AgeRatingContentDescriptionType
);
igdb_ids_route!(
	"/igdb/age-rating-content-description-types",
	get_igdb_age_rating_content_description_types_by_ids,
	get_age_rating_content_description_type_by_id_cached,
	AgeRatingContentDescriptionType
);

igdb_id_route!(
	"/igdb/age-rating-organization",
	get_igdb_age_rating_organization_by_id,
	get_age_rating_organization_by_id_cached,
	AgeRatingOrganization
);
igdb_ids_route!(
	"/igdb/age-rating-organizations",
	get_igdb_age_rating_organizations_by_ids,
	get_age_rating_organization_by_id_cached,
	AgeRatingOrganization
);

igdb_id_route!(
	"/igdb/company-status",
	get_igdb_company_status_by_id,
	get_company_status_by_id_cached,
	CompanyStatus
);
igdb_ids_route!(
	"/igdb/company-statuses",
	get_igdb_company_statuses_by_ids,
	get_company_status_by_id_cached,
	CompanyStatus
);

igdb_id_route!(
	"/igdb/date-format",
	get_igdb_date_format_by_id,
	get_date_format_by_id_cached,
	DateFormat
);
igdb_ids_route!(
	"/igdb/date-formats",
	get_igdb_date_formats_by_ids,
	get_date_format_by_id_cached,
	DateFormat
);

igdb_id_route!(
	"/igdb/external-game-source",
	get_igdb_external_game_source_by_id,
	get_external_game_source_by_id_cached,
	ExternalGameSource
);
igdb_ids_route!(
	"/igdb/external-game-sources",
	get_igdb_external_game_sources_by_ids,
	get_external_game_source_by_id_cached,
	ExternalGameSource
);

igdb_id_route!(
	"/igdb/game-release-format",
	get_igdb_game_release_format_by_id,
	get_game_release_format_by_id_cached,
	GameReleaseFormat
);
igdb_ids_route!(
	"/igdb/game-release-formats",
	get_igdb_game_release_formats_by_ids,
	get_game_release_format_by_id_cached,
	GameReleaseFormat
);

igdb_id_route!(
	"/igdb/game-status",
	get_igdb_game_status_by_id,
	get_game_status_by_id_cached,
	GameStatus
);
igdb_ids_route!(
	"/igdb/game-statuses",
	get_igdb_game_statuses_by_ids,
	get_game_status_by_id_cached,
	GameStatus
);

igdb_id_route!(
	"/igdb/game-type",
	get_igdb_game_type_by_id,
	get_game_type_by_id_cached,
	GameType
);
igdb_ids_route!(
	"/igdb/game-types",
	get_igdb_game_types_by_ids,
	get_game_type_by_id_cached,
	GameType
);

igdb_id_route!(
	"/igdb/platform-type",
	get_igdb_platform_type_by_id,
	get_platform_type_by_id_cached,
	PlatformType
);
igdb_ids_route!(
	"/igdb/platform-types",
	get_igdb_platform_types_by_ids,
	get_platform_type_by_id_cached,
	PlatformType
);

igdb_id_route!(
	"/igdb/release-date-region",
	get_igdb_release_date_region_by_id,
	get_release_date_region_by_id_cached,
	ReleaseDateRegion
);
igdb_ids_route!(
	"/igdb/release-date-regions",
	get_igdb_release_date_regions_by_ids,
	get_release_date_region_by_id_cached,
	ReleaseDateRegion
);

igdb_id_route!(
	"/igdb/website-type",
	get_igdb_website_type_by_id,
	get_website_type_by_id_cached,
	WebsiteType
);
igdb_ids_route!(
	"/igdb/website-types",
	get_igdb_website_types_by_ids,
	get_website_type_by_id_cached,
	WebsiteType
);

igdb_id_route!(
	"/igdb/artwork-type",
	get_igdb_artwork_type_by_id,
	get_artwork_type_by_id_cached,
	ArtworkType
);
igdb_ids_route!(
	"/igdb/artwork-types",
	get_igdb_artwork_types_by_ids,
	get_artwork_type_by_id_cached,
	ArtworkType
);

igdb_id_route!(
	"/igdb/character-mug-shot",
	get_igdb_character_mug_shot_by_id,
	get_character_mug_shot_by_id_cached,
	CharacterMugShot
);
igdb_ids_route!(
	"/igdb/character-mug-shots",
	get_igdb_character_mug_shots_by_ids,
	get_character_mug_shot_by_id_cached,
	CharacterMugShot
);

igdb_id_route!(
	"/igdb/company-size",
	get_igdb_company_size_by_id,
	get_company_size_by_id_cached,
	CompanySize
);
igdb_ids_route!(
	"/igdb/company-sizes",
	get_igdb_company_sizes_by_ids,
	get_company_size_by_id_cached,
	CompanySize
);

igdb_id_route!(
	"/igdb/company-type",
	get_igdb_company_type_by_id,
	get_company_type_by_id_cached,
	CompanyType
);
igdb_ids_route!(
	"/igdb/company-types",
	get_igdb_company_types_by_ids,
	get_company_type_by_id_cached,
	CompanyType
);

igdb_id_route!(
	"/igdb/company-type-history",
	get_igdb_company_type_history_by_id,
	get_company_type_history_by_id_cached,
	CompanyTypeHistory
);
igdb_ids_route!(
	"/igdb/company-type-histories",
	get_igdb_company_type_histories_by_ids,
	get_company_type_history_by_id_cached,
	CompanyTypeHistory
);

igdb_id_route!(
	"/igdb/entity-type",
	get_igdb_entity_type_by_id,
	get_entity_type_by_id_cached,
	EntityType
);
igdb_ids_route!(
	"/igdb/entity-types",
	get_igdb_entity_types_by_ids,
	get_entity_type_by_id_cached,
	EntityType
);

igdb_id_route!(
	"/igdb/game-time-to-beat",
	get_igdb_game_time_to_beat_by_id,
	get_game_time_to_beat_by_id_cached,
	GameTimeToBeat
);
igdb_ids_route!(
	"/igdb/game-time-to-beats",
	get_igdb_game_time_to_beats_by_ids,
	get_game_time_to_beat_by_id_cached,
	GameTimeToBeat
);

igdb_id_route!(
	"/igdb/report",
	get_igdb_report_by_id,
	get_report_by_id_cached,
	Report
);
igdb_ids_route!(
	"/igdb/reports",
	get_igdb_reports_by_ids,
	get_report_by_id_cached,
	Report
);

igdb_id_route!(
	"/igdb/report-type",
	get_igdb_report_type_by_id,
	get_report_type_by_id_cached,
	ReportType
);
igdb_ids_route!(
	"/igdb/report-types",
	get_igdb_report_types_by_ids,
	get_report_type_by_id_cached,
	ReportType
);

igdb_id_route!(
	"/igdb/character",
	get_igdb_character_by_id,
	get_character_by_id_cached,
	Character
);
igdb_ids_route!(
	"/igdb/characters",
	get_igdb_characters_by_ids,
	get_character_by_id_cached,
	Character
);

igdb_id_route!(
	"/igdb/character-gender",
	get_igdb_character_gender_by_id,
	get_character_gender_by_id_cached,
	CharacterGender
);
igdb_ids_route!(
	"/igdb/character-genders",
	get_igdb_character_genders_by_ids,
	get_character_gender_by_id_cached,
	CharacterGender
);

igdb_id_route!(
	"/igdb/character-species",
	get_igdb_character_species_by_id,
	get_character_species_by_id_cached,
	CharacterSpecies
);
igdb_ids_route!(
	"/igdb/character-species-list",
	get_igdb_character_species_by_ids,
	get_character_species_by_id_cached,
	CharacterSpecies
);

igdb_id_route!(
	"/igdb/collection-membership",
	get_igdb_collection_membership_by_id,
	get_collection_membership_by_id_cached,
	CollectionMembership
);
igdb_ids_route!(
	"/igdb/collection-memberships",
	get_igdb_collection_memberships_by_ids,
	get_collection_membership_by_id_cached,
	CollectionMembership
);

igdb_id_route!(
	"/igdb/collection-membership-type",
	get_igdb_collection_membership_type_by_id,
	get_collection_membership_type_by_id_cached,
	CollectionMembershipType
);
igdb_ids_route!(
	"/igdb/collection-membership-types",
	get_igdb_collection_membership_types_by_ids,
	get_collection_membership_type_by_id_cached,
	CollectionMembershipType
);

igdb_id_route!(
	"/igdb/collection-relation",
	get_igdb_collection_relation_by_id,
	get_collection_relation_by_id_cached,
	CollectionRelation
);
igdb_ids_route!(
	"/igdb/collection-relations",
	get_igdb_collection_relations_by_ids,
	get_collection_relation_by_id_cached,
	CollectionRelation
);

igdb_id_route!(
	"/igdb/collection-relation-type",
	get_igdb_collection_relation_type_by_id,
	get_collection_relation_type_by_id_cached,
	CollectionRelationType
);
igdb_ids_route!(
	"/igdb/collection-relation-types",
	get_igdb_collection_relation_types_by_ids,
	get_collection_relation_type_by_id_cached,
	CollectionRelationType
);

igdb_id_route!(
	"/igdb/collection-type",
	get_igdb_collection_type_by_id,
	get_collection_type_by_id_cached,
	CollectionType
);
igdb_ids_route!(
	"/igdb/collection-types",
	get_igdb_collection_types_by_ids,
	get_collection_type_by_id_cached,
	CollectionType
);

igdb_id_route!(
	"/igdb/company",
	get_igdb_company_by_id,
	get_company_by_id_cached,
	Company
);
igdb_ids_route!(
	"/igdb/companies",
	get_igdb_companies_by_ids,
	get_company_by_id_cached,
	Company
);

igdb_id_route!(
	"/igdb/company-logo",
	get_igdb_company_logo_by_id,
	get_company_logo_by_id_cached,
	CompanyLogo
);
igdb_ids_route!(
	"/igdb/company-logos",
	get_igdb_company_logos_by_ids,
	get_company_logo_by_id_cached,
	CompanyLogo
);

igdb_id_route!(
	"/igdb/company-website",
	get_igdb_company_website_by_id,
	get_company_website_by_id_cached,
	CompanyWebsite
);
igdb_ids_route!(
	"/igdb/company-websites",
	get_igdb_company_websites_by_ids,
	get_company_website_by_id_cached,
	CompanyWebsite
);

igdb_id_route!(
	"/igdb/event",
	get_igdb_event_by_id,
	get_event_by_id_cached,
	Event
);
igdb_ids_route!(
	"/igdb/events",
	get_igdb_events_by_ids,
	get_event_by_id_cached,
	Event
);

igdb_id_route!(
	"/igdb/event-logo",
	get_igdb_event_logo_by_id,
	get_event_logo_by_id_cached,
	EventLogo
);
igdb_ids_route!(
	"/igdb/event-logos",
	get_igdb_event_logos_by_ids,
	get_event_logo_by_id_cached,
	EventLogo
);

igdb_id_route!(
	"/igdb/event-network",
	get_igdb_event_network_by_id,
	get_event_network_by_id_cached,
	EventNetwork
);
igdb_ids_route!(
	"/igdb/event-networks",
	get_igdb_event_networks_by_ids,
	get_event_network_by_id_cached,
	EventNetwork
);

igdb_id_route!(
	"/igdb/game-engine",
	get_igdb_game_engine_by_id,
	get_game_engine_by_id_cached,
	GameEngine
);
igdb_ids_route!(
	"/igdb/game-engines",
	get_igdb_game_engines_by_ids,
	get_game_engine_by_id_cached,
	GameEngine
);

igdb_id_route!(
	"/igdb/game-engine-logo",
	get_igdb_game_engine_logo_by_id,
	get_game_engine_logo_by_id_cached,
	GameEngineLogo
);
igdb_ids_route!(
	"/igdb/game-engine-logos",
	get_igdb_game_engine_logos_by_ids,
	get_game_engine_logo_by_id_cached,
	GameEngineLogo
);

igdb_id_route!(
	"/igdb/game-localization",
	get_igdb_game_localization_by_id,
	get_game_localization_by_id_cached,
	GameLocalization
);
igdb_ids_route!(
	"/igdb/game-localizations",
	get_igdb_game_localizations_by_ids,
	get_game_localization_by_id_cached,
	GameLocalization
);

igdb_id_route!(
	"/igdb/game-mode",
	get_igdb_game_mode_by_id,
	get_game_mode_by_id_cached,
	GameMode
);
igdb_ids_route!(
	"/igdb/game-modes",
	get_igdb_game_modes_by_ids,
	get_game_mode_by_id_cached,
	GameMode
);

igdb_id_route!(
	"/igdb/game-version",
	get_igdb_game_version_by_id,
	get_game_version_by_id_cached,
	GameVersion
);
igdb_ids_route!(
	"/igdb/game-versions",
	get_igdb_game_versions_by_ids,
	get_game_version_by_id_cached,
	GameVersion
);

igdb_id_route!(
	"/igdb/game-version-feature",
	get_igdb_game_version_feature_by_id,
	get_game_version_feature_by_id_cached,
	GameVersionFeature
);
igdb_ids_route!(
	"/igdb/game-version-features",
	get_igdb_game_version_features_by_ids,
	get_game_version_feature_by_id_cached,
	GameVersionFeature
);

igdb_id_route!(
	"/igdb/game-version-feature-value",
	get_igdb_game_version_feature_value_by_id,
	get_game_version_feature_value_by_id_cached,
	GameVersionFeatureValue
);
igdb_ids_route!(
	"/igdb/game-version-feature-values",
	get_igdb_game_version_feature_values_by_ids,
	get_game_version_feature_value_by_id_cached,
	GameVersionFeatureValue
);

igdb_id_route!(
	"/igdb/game-video",
	get_igdb_game_video_by_id,
	get_game_video_by_id_cached,
	GameVideo
);
igdb_ids_route!(
	"/igdb/game-videos",
	get_igdb_game_videos_by_ids,
	get_game_video_by_id_cached,
	GameVideo
);

igdb_id_route!(
	"/igdb/involved-company",
	get_igdb_involved_company_by_id,
	get_involved_company_by_id_cached,
	InvolvedCompany
);
igdb_ids_route!(
	"/igdb/involved-companies",
	get_igdb_involved_companies_by_ids,
	get_involved_company_by_id_cached,
	InvolvedCompany
);

igdb_id_route!(
	"/igdb/keyword",
	get_igdb_keyword_by_id,
	get_keyword_by_id_cached,
	Keyword
);
igdb_ids_route!(
	"/igdb/keywords",
	get_igdb_keywords_by_ids,
	get_keyword_by_id_cached,
	Keyword
);

igdb_id_route!(
	"/igdb/language",
	get_igdb_language_by_id,
	get_language_by_id_cached,
	Language
);
igdb_ids_route!(
	"/igdb/languages",
	get_igdb_languages_by_ids,
	get_language_by_id_cached,
	Language
);

igdb_id_route!(
	"/igdb/language-support",
	get_igdb_language_support_by_id,
	get_language_support_by_id_cached,
	LanguageSupport
);
igdb_ids_route!(
	"/igdb/language-supports",
	get_igdb_language_supports_by_ids,
	get_language_support_by_id_cached,
	LanguageSupport
);

igdb_id_route!(
	"/igdb/language-support-type",
	get_igdb_language_support_type_by_id,
	get_language_support_type_by_id_cached,
	LanguageSupportType
);
igdb_ids_route!(
	"/igdb/language-support-types",
	get_igdb_language_support_types_by_ids,
	get_language_support_type_by_id_cached,
	LanguageSupportType
);

igdb_id_route!(
	"/igdb/multiplayer-mode",
	get_igdb_multiplayer_mode_by_id,
	get_multiplayer_mode_by_id_cached,
	MultiplayerMode
);
igdb_ids_route!(
	"/igdb/multiplayer-modes",
	get_igdb_multiplayer_modes_by_ids,
	get_multiplayer_mode_by_id_cached,
	MultiplayerMode
);

igdb_id_route!(
	"/igdb/network-type",
	get_igdb_network_type_by_id,
	get_network_type_by_id_cached,
	NetworkType
);
igdb_ids_route!(
	"/igdb/network-types",
	get_igdb_network_types_by_ids,
	get_network_type_by_id_cached,
	NetworkType
);

igdb_id_route!(
	"/igdb/platform",
	get_igdb_platform_by_id,
	get_platform_by_id_cached,
	Platform
);
igdb_ids_route!(
	"/igdb/platforms",
	get_igdb_platforms_by_ids,
	get_platform_by_id_cached,
	Platform
);

igdb_id_route!(
	"/igdb/platform-family",
	get_igdb_platform_family_by_id,
	get_platform_family_by_id_cached,
	PlatformFamily
);
igdb_ids_route!(
	"/igdb/platform-families",
	get_igdb_platform_families_by_ids,
	get_platform_family_by_id_cached,
	PlatformFamily
);

igdb_id_route!(
	"/igdb/platform-logo",
	get_igdb_platform_logo_by_id,
	get_platform_logo_by_id_cached,
	PlatformLogo
);
igdb_ids_route!(
	"/igdb/platform-logos",
	get_igdb_platform_logos_by_ids,
	get_platform_logo_by_id_cached,
	PlatformLogo
);

igdb_id_route!(
	"/igdb/platform-version",
	get_igdb_platform_version_by_id,
	get_platform_version_by_id_cached,
	PlatformVersion
);
igdb_ids_route!(
	"/igdb/platform-versions",
	get_igdb_platform_versions_by_ids,
	get_platform_version_by_id_cached,
	PlatformVersion
);

igdb_id_route!(
	"/igdb/platform-version-company",
	get_igdb_platform_version_company_by_id,
	get_platform_version_company_by_id_cached,
	PlatformVersionCompany
);
igdb_ids_route!(
	"/igdb/platform-version-companies",
	get_igdb_platform_version_companies_by_ids,
	get_platform_version_company_by_id_cached,
	PlatformVersionCompany
);

igdb_id_route!(
	"/igdb/platform-version-release-date",
	get_igdb_platform_version_release_date_by_id,
	get_platform_version_release_date_by_id_cached,
	PlatformVersionReleaseDate
);
igdb_ids_route!(
	"/igdb/platform-version-release-dates",
	get_igdb_platform_version_release_dates_by_ids,
	get_platform_version_release_date_by_id_cached,
	PlatformVersionReleaseDate
);

igdb_id_route!(
	"/igdb/platform-website",
	get_igdb_platform_website_by_id,
	get_platform_website_by_id_cached,
	PlatformWebsite
);
igdb_ids_route!(
	"/igdb/platform-websites",
	get_igdb_platform_websites_by_ids,
	get_platform_website_by_id_cached,
	PlatformWebsite
);

igdb_id_route!(
	"/igdb/player-perspective",
	get_igdb_player_perspective_by_id,
	get_player_perspective_by_id_cached,
	PlayerPerspective
);
igdb_ids_route!(
	"/igdb/player-perspectives",
	get_igdb_player_perspectives_by_ids,
	get_player_perspective_by_id_cached,
	PlayerPerspective
);

igdb_id_route!(
	"/igdb/popularity-primitive",
	get_igdb_popularity_primitive_by_id,
	get_popularity_primitive_by_id_cached,
	PopularityPrimitive
);
igdb_ids_route!(
	"/igdb/popularity-primitives",
	get_igdb_popularity_primitives_by_ids,
	get_popularity_primitive_by_id_cached,
	PopularityPrimitive
);

igdb_id_route!(
	"/igdb/popularity-type",
	get_igdb_popularity_type_by_id,
	get_popularity_type_by_id_cached,
	PopularityType
);
igdb_ids_route!(
	"/igdb/popularity-types",
	get_igdb_popularity_types_by_ids,
	get_popularity_type_by_id_cached,
	PopularityType
);

igdb_id_route!(
	"/igdb/region",
	get_igdb_region_by_id,
	get_region_by_id_cached,
	Region
);
igdb_ids_route!(
	"/igdb/regions",
	get_igdb_regions_by_ids,
	get_region_by_id_cached,
	Region
);

igdb_id_route!(
	"/igdb/release-date",
	get_igdb_release_date_by_id,
	get_release_date_by_id_cached,
	ReleaseDate
);
igdb_ids_route!(
	"/igdb/release-dates",
	get_igdb_release_dates_by_ids,
	get_release_date_by_id_cached,
	ReleaseDate
);

igdb_id_route!(
	"/igdb/release-date-status",
	get_igdb_release_date_status_by_id,
	get_release_date_status_by_id_cached,
	ReleaseDateStatus
);
igdb_ids_route!(
	"/igdb/release-date-statuses",
	get_igdb_release_date_statuses_by_ids,
	get_release_date_status_by_id_cached,
	ReleaseDateStatus
);

igdb_id_route!(
	"/igdb/screenshot",
	get_igdb_screenshot_by_id,
	get_screenshot_by_id_cached,
	Screenshot
);
igdb_ids_route!(
	"/igdb/screenshots",
	get_igdb_screenshots_by_ids,
	get_screenshot_by_id_cached,
	Screenshot
);

igdb_id_route!(
	"/igdb/theme",
	get_igdb_theme_by_id,
	get_theme_by_id_cached,
	Theme
);
igdb_ids_route!(
	"/igdb/themes",
	get_igdb_themes_by_ids,
	get_theme_by_id_cached,
	Theme
);

igdb_id_route!(
	"/igdb/website",
	get_igdb_website_by_id,
	get_website_by_id_cached,
	Website
);
igdb_ids_route!(
	"/igdb/websites",
	get_igdb_websites_by_ids,
	get_website_by_id_cached,
	Website
);
