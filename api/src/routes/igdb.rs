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

/// IGDB-prefilled thin wrapper around `$crate::__provider_entity_routes_impl`.
/// `$singular_summary`/`$plural_summary` are full doc sentences (see that
/// macro's doc comment for why they cannot be built from the entity name at
/// macro-expansion time).
macro_rules! igdb_entity_routes {
	(
		$singular_route:literal,
		$singular_fn:ident,
		$plural_route:literal,
		$plural_fn:ident,
		$cached_fn:ident,
		$model:ty,
		$singular_summary:literal,
		$plural_summary:literal
	) => {
		$crate::__provider_entity_routes_impl!(
			IgdbClient,
			"IGDB",
			$singular_route,
			$singular_fn,
			$plural_route,
			$plural_fn,
			$cached_fn,
			$model,
			$singular_summary,
			$plural_summary
		);
	};
}

/// Returns an IGDB game by id or slug.
///
/// Supply either `id` or `slug`. If both are present, `id` is used.
#[utoipa::path(
	get,
	tag = "IGDB",
	params(SlugIdQuery),
	responses(
		(status = 200, description = "The matched IGDB game", body = Game),
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
		return Ok(HttpResponse::BadRequest().body("either slug or id must be provided"));
	};

	if let Some(game) = response {
		Ok(HttpResponse::Ok().json(game))
	} else {
		Ok(HttpResponse::NotFound().finish())
	}
}

/// Looks up many IGDB games by id in one request.
#[utoipa::path(
	get,
	tag = "IGDB",
	params(IdsQuery),
	responses(
		(status = 200, description = "The matched IGDB games", body = Vec<Game>)
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

/// Searches IGDB games by name, ordered by relevance.
#[utoipa::path(
	get,
	tag = "IGDB",
	params(SearchQuery),
	responses(
		(status = 200, description = "Matching IGDB games ordered by relevance", body = Vec<Game>)
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

igdb_entity_routes!(
	"/igdb/age-rating",
	get_igdb_age_rating_by_id,
	"/igdb/age-ratings",
	get_igdb_age_ratings_by_ids,
	get_age_rating_by_id_cached,
	AgeRating,
	"Returns one IGDB AgeRating by id.",
	"Looks up many IGDB AgeRating records by id in one request."
);

igdb_entity_routes!(
	"/igdb/alternative-name",
	get_igdb_alternative_name_by_id,
	"/igdb/alternative-names",
	get_igdb_alternative_names_by_ids,
	get_alternative_name_by_id_cached,
	AlternativeName,
	"Returns one IGDB AlternativeName by id.",
	"Looks up many IGDB AlternativeName records by id in one request."
);

igdb_entity_routes!(
	"/igdb/artwork",
	get_igdb_artwork_by_id,
	"/igdb/artworks",
	get_igdb_artworks_by_ids,
	get_artwork_by_id_cached,
	Artwork,
	"Returns one IGDB Artwork by id.",
	"Looks up many IGDB Artwork records by id in one request."
);

igdb_entity_routes!(
	"/igdb/collection",
	get_igdb_collection_by_id,
	"/igdb/collections",
	get_igdb_collections_by_ids,
	get_collection_by_id_cached,
	Collection,
	"Returns one IGDB Collection by id.",
	"Looks up many IGDB Collection records by id in one request."
);

igdb_entity_routes!(
	"/igdb/cover",
	get_igdb_cover_by_id,
	"/igdb/covers",
	get_igdb_covers_by_ids,
	get_cover_by_id_cached,
	Cover,
	"Returns one IGDB Cover by id.",
	"Looks up many IGDB Cover records by id in one request."
);

igdb_entity_routes!(
	"/igdb/external-game",
	get_igdb_external_game_by_id,
	"/igdb/external-games",
	get_igdb_external_games_by_ids,
	get_external_game_by_id_cached,
	ExternalGame,
	"Returns one IGDB ExternalGame by id.",
	"Looks up many IGDB ExternalGame records by id in one request."
);

igdb_entity_routes!(
	"/igdb/franchise",
	get_igdb_franchise_by_id,
	"/igdb/franchises",
	get_igdb_franchises_by_ids,
	get_franchise_by_id_cached,
	Franchise,
	"Returns one IGDB Franchise by id.",
	"Looks up many IGDB Franchise records by id in one request."
);

igdb_entity_routes!(
	"/igdb/genre",
	get_igdb_genre_by_id,
	"/igdb/genres",
	get_igdb_genres_by_ids,
	get_genre_by_id_cached,
	Genre,
	"Returns one IGDB Genre by id.",
	"Looks up many IGDB Genre records by id in one request."
);

igdb_entity_routes!(
	"/igdb/age-rating-category",
	get_igdb_age_rating_category_by_id,
	"/igdb/age-rating-categories",
	get_igdb_age_rating_categories_by_ids,
	get_age_rating_category_by_id_cached,
	AgeRatingCategory,
	"Returns one IGDB AgeRatingCategory by id.",
	"Looks up many IGDB AgeRatingCategory records by id in one request."
);

igdb_entity_routes!(
	"/igdb/age-rating-content-description-v2",
	get_igdb_age_rating_content_description_v2_by_id,
	"/igdb/age-rating-content-descriptions-v2",
	get_igdb_age_rating_content_descriptions_v2_by_ids,
	get_age_rating_content_description_v2_by_id_cached,
	AgeRatingContentDescriptionV2,
	"Returns one IGDB AgeRatingContentDescriptionV2 by id.",
	"Looks up many IGDB AgeRatingContentDescriptionV2 records by id in one request."
);

igdb_entity_routes!(
	"/igdb/age-rating-content-description-type",
	get_igdb_age_rating_content_description_type_by_id,
	"/igdb/age-rating-content-description-types",
	get_igdb_age_rating_content_description_types_by_ids,
	get_age_rating_content_description_type_by_id_cached,
	AgeRatingContentDescriptionType,
	"Returns one IGDB AgeRatingContentDescriptionType by id.",
	"Looks up many IGDB AgeRatingContentDescriptionType records by id in one request."
);

igdb_entity_routes!(
	"/igdb/age-rating-organization",
	get_igdb_age_rating_organization_by_id,
	"/igdb/age-rating-organizations",
	get_igdb_age_rating_organizations_by_ids,
	get_age_rating_organization_by_id_cached,
	AgeRatingOrganization,
	"Returns one IGDB AgeRatingOrganization by id.",
	"Looks up many IGDB AgeRatingOrganization records by id in one request."
);

igdb_entity_routes!(
	"/igdb/company-status",
	get_igdb_company_status_by_id,
	"/igdb/company-statuses",
	get_igdb_company_statuses_by_ids,
	get_company_status_by_id_cached,
	CompanyStatus,
	"Returns one IGDB CompanyStatus by id.",
	"Looks up many IGDB CompanyStatus records by id in one request."
);

igdb_entity_routes!(
	"/igdb/date-format",
	get_igdb_date_format_by_id,
	"/igdb/date-formats",
	get_igdb_date_formats_by_ids,
	get_date_format_by_id_cached,
	DateFormat,
	"Returns one IGDB DateFormat by id.",
	"Looks up many IGDB DateFormat records by id in one request."
);

igdb_entity_routes!(
	"/igdb/external-game-source",
	get_igdb_external_game_source_by_id,
	"/igdb/external-game-sources",
	get_igdb_external_game_sources_by_ids,
	get_external_game_source_by_id_cached,
	ExternalGameSource,
	"Returns one IGDB ExternalGameSource by id.",
	"Looks up many IGDB ExternalGameSource records by id in one request."
);

igdb_entity_routes!(
	"/igdb/game-release-format",
	get_igdb_game_release_format_by_id,
	"/igdb/game-release-formats",
	get_igdb_game_release_formats_by_ids,
	get_game_release_format_by_id_cached,
	GameReleaseFormat,
	"Returns one IGDB GameReleaseFormat by id.",
	"Looks up many IGDB GameReleaseFormat records by id in one request."
);

igdb_entity_routes!(
	"/igdb/game-status",
	get_igdb_game_status_by_id,
	"/igdb/game-statuses",
	get_igdb_game_statuses_by_ids,
	get_game_status_by_id_cached,
	GameStatus,
	"Returns one IGDB GameStatus by id.",
	"Looks up many IGDB GameStatus records by id in one request."
);

igdb_entity_routes!(
	"/igdb/game-type",
	get_igdb_game_type_by_id,
	"/igdb/game-types",
	get_igdb_game_types_by_ids,
	get_game_type_by_id_cached,
	GameType,
	"Returns one IGDB GameType by id.",
	"Looks up many IGDB GameType records by id in one request."
);

igdb_entity_routes!(
	"/igdb/platform-type",
	get_igdb_platform_type_by_id,
	"/igdb/platform-types",
	get_igdb_platform_types_by_ids,
	get_platform_type_by_id_cached,
	PlatformType,
	"Returns one IGDB PlatformType by id.",
	"Looks up many IGDB PlatformType records by id in one request."
);

igdb_entity_routes!(
	"/igdb/release-date-region",
	get_igdb_release_date_region_by_id,
	"/igdb/release-date-regions",
	get_igdb_release_date_regions_by_ids,
	get_release_date_region_by_id_cached,
	ReleaseDateRegion,
	"Returns one IGDB ReleaseDateRegion by id.",
	"Looks up many IGDB ReleaseDateRegion records by id in one request."
);

igdb_entity_routes!(
	"/igdb/website-type",
	get_igdb_website_type_by_id,
	"/igdb/website-types",
	get_igdb_website_types_by_ids,
	get_website_type_by_id_cached,
	WebsiteType,
	"Returns one IGDB WebsiteType by id.",
	"Looks up many IGDB WebsiteType records by id in one request."
);

igdb_entity_routes!(
	"/igdb/artwork-type",
	get_igdb_artwork_type_by_id,
	"/igdb/artwork-types",
	get_igdb_artwork_types_by_ids,
	get_artwork_type_by_id_cached,
	ArtworkType,
	"Returns one IGDB ArtworkType by id.",
	"Looks up many IGDB ArtworkType records by id in one request."
);

igdb_entity_routes!(
	"/igdb/character-mug-shot",
	get_igdb_character_mug_shot_by_id,
	"/igdb/character-mug-shots",
	get_igdb_character_mug_shots_by_ids,
	get_character_mug_shot_by_id_cached,
	CharacterMugShot,
	"Returns one IGDB CharacterMugShot by id.",
	"Looks up many IGDB CharacterMugShot records by id in one request."
);

igdb_entity_routes!(
	"/igdb/company-size",
	get_igdb_company_size_by_id,
	"/igdb/company-sizes",
	get_igdb_company_sizes_by_ids,
	get_company_size_by_id_cached,
	CompanySize,
	"Returns one IGDB CompanySize by id.",
	"Looks up many IGDB CompanySize records by id in one request."
);

igdb_entity_routes!(
	"/igdb/company-type",
	get_igdb_company_type_by_id,
	"/igdb/company-types",
	get_igdb_company_types_by_ids,
	get_company_type_by_id_cached,
	CompanyType,
	"Returns one IGDB CompanyType by id.",
	"Looks up many IGDB CompanyType records by id in one request."
);

igdb_entity_routes!(
	"/igdb/company-type-history",
	get_igdb_company_type_history_by_id,
	"/igdb/company-type-histories",
	get_igdb_company_type_histories_by_ids,
	get_company_type_history_by_id_cached,
	CompanyTypeHistory,
	"Returns one IGDB CompanyTypeHistory by id.",
	"Looks up many IGDB CompanyTypeHistory records by id in one request."
);

igdb_entity_routes!(
	"/igdb/entity-type",
	get_igdb_entity_type_by_id,
	"/igdb/entity-types",
	get_igdb_entity_types_by_ids,
	get_entity_type_by_id_cached,
	EntityType,
	"Returns one IGDB EntityType by id.",
	"Looks up many IGDB EntityType records by id in one request."
);

igdb_entity_routes!(
	"/igdb/game-time-to-beat",
	get_igdb_game_time_to_beat_by_id,
	"/igdb/game-time-to-beats",
	get_igdb_game_time_to_beats_by_ids,
	get_game_time_to_beat_by_id_cached,
	GameTimeToBeat,
	"Returns one IGDB GameTimeToBeat by id.",
	"Looks up many IGDB GameTimeToBeat records by id in one request."
);

igdb_entity_routes!(
	"/igdb/report",
	get_igdb_report_by_id,
	"/igdb/reports",
	get_igdb_reports_by_ids,
	get_report_by_id_cached,
	Report,
	"Returns one IGDB Report by id.",
	"Looks up many IGDB Report records by id in one request."
);

igdb_entity_routes!(
	"/igdb/report-type",
	get_igdb_report_type_by_id,
	"/igdb/report-types",
	get_igdb_report_types_by_ids,
	get_report_type_by_id_cached,
	ReportType,
	"Returns one IGDB ReportType by id.",
	"Looks up many IGDB ReportType records by id in one request."
);

igdb_entity_routes!(
	"/igdb/character",
	get_igdb_character_by_id,
	"/igdb/characters",
	get_igdb_characters_by_ids,
	get_character_by_id_cached,
	Character,
	"Returns one IGDB Character by id.",
	"Looks up many IGDB Character records by id in one request."
);

igdb_entity_routes!(
	"/igdb/character-gender",
	get_igdb_character_gender_by_id,
	"/igdb/character-genders",
	get_igdb_character_genders_by_ids,
	get_character_gender_by_id_cached,
	CharacterGender,
	"Returns one IGDB CharacterGender by id.",
	"Looks up many IGDB CharacterGender records by id in one request."
);

igdb_entity_routes!(
	"/igdb/character-species",
	get_igdb_character_species_by_id,
	"/igdb/character-species-list",
	get_igdb_character_species_by_ids,
	get_character_species_by_id_cached,
	CharacterSpecies,
	"Returns one IGDB CharacterSpecies by id.",
	"Looks up many IGDB CharacterSpecies records by id in one request."
);

igdb_entity_routes!(
	"/igdb/collection-membership",
	get_igdb_collection_membership_by_id,
	"/igdb/collection-memberships",
	get_igdb_collection_memberships_by_ids,
	get_collection_membership_by_id_cached,
	CollectionMembership,
	"Returns one IGDB CollectionMembership by id.",
	"Looks up many IGDB CollectionMembership records by id in one request."
);

igdb_entity_routes!(
	"/igdb/collection-membership-type",
	get_igdb_collection_membership_type_by_id,
	"/igdb/collection-membership-types",
	get_igdb_collection_membership_types_by_ids,
	get_collection_membership_type_by_id_cached,
	CollectionMembershipType,
	"Returns one IGDB CollectionMembershipType by id.",
	"Looks up many IGDB CollectionMembershipType records by id in one request."
);

igdb_entity_routes!(
	"/igdb/collection-relation",
	get_igdb_collection_relation_by_id,
	"/igdb/collection-relations",
	get_igdb_collection_relations_by_ids,
	get_collection_relation_by_id_cached,
	CollectionRelation,
	"Returns one IGDB CollectionRelation by id.",
	"Looks up many IGDB CollectionRelation records by id in one request."
);

igdb_entity_routes!(
	"/igdb/collection-relation-type",
	get_igdb_collection_relation_type_by_id,
	"/igdb/collection-relation-types",
	get_igdb_collection_relation_types_by_ids,
	get_collection_relation_type_by_id_cached,
	CollectionRelationType,
	"Returns one IGDB CollectionRelationType by id.",
	"Looks up many IGDB CollectionRelationType records by id in one request."
);

igdb_entity_routes!(
	"/igdb/collection-type",
	get_igdb_collection_type_by_id,
	"/igdb/collection-types",
	get_igdb_collection_types_by_ids,
	get_collection_type_by_id_cached,
	CollectionType,
	"Returns one IGDB CollectionType by id.",
	"Looks up many IGDB CollectionType records by id in one request."
);

igdb_entity_routes!(
	"/igdb/company",
	get_igdb_company_by_id,
	"/igdb/companies",
	get_igdb_companies_by_ids,
	get_company_by_id_cached,
	Company,
	"Returns one IGDB Company by id.",
	"Looks up many IGDB Company records by id in one request."
);

igdb_entity_routes!(
	"/igdb/company-logo",
	get_igdb_company_logo_by_id,
	"/igdb/company-logos",
	get_igdb_company_logos_by_ids,
	get_company_logo_by_id_cached,
	CompanyLogo,
	"Returns one IGDB CompanyLogo by id.",
	"Looks up many IGDB CompanyLogo records by id in one request."
);

igdb_entity_routes!(
	"/igdb/company-website",
	get_igdb_company_website_by_id,
	"/igdb/company-websites",
	get_igdb_company_websites_by_ids,
	get_company_website_by_id_cached,
	CompanyWebsite,
	"Returns one IGDB CompanyWebsite by id.",
	"Looks up many IGDB CompanyWebsite records by id in one request."
);

igdb_entity_routes!(
	"/igdb/event",
	get_igdb_event_by_id,
	"/igdb/events",
	get_igdb_events_by_ids,
	get_event_by_id_cached,
	Event,
	"Returns one IGDB Event by id.",
	"Looks up many IGDB Event records by id in one request."
);

igdb_entity_routes!(
	"/igdb/event-logo",
	get_igdb_event_logo_by_id,
	"/igdb/event-logos",
	get_igdb_event_logos_by_ids,
	get_event_logo_by_id_cached,
	EventLogo,
	"Returns one IGDB EventLogo by id.",
	"Looks up many IGDB EventLogo records by id in one request."
);

igdb_entity_routes!(
	"/igdb/event-network",
	get_igdb_event_network_by_id,
	"/igdb/event-networks",
	get_igdb_event_networks_by_ids,
	get_event_network_by_id_cached,
	EventNetwork,
	"Returns one IGDB EventNetwork by id.",
	"Looks up many IGDB EventNetwork records by id in one request."
);

igdb_entity_routes!(
	"/igdb/game-engine",
	get_igdb_game_engine_by_id,
	"/igdb/game-engines",
	get_igdb_game_engines_by_ids,
	get_game_engine_by_id_cached,
	GameEngine,
	"Returns one IGDB GameEngine by id.",
	"Looks up many IGDB GameEngine records by id in one request."
);

igdb_entity_routes!(
	"/igdb/game-engine-logo",
	get_igdb_game_engine_logo_by_id,
	"/igdb/game-engine-logos",
	get_igdb_game_engine_logos_by_ids,
	get_game_engine_logo_by_id_cached,
	GameEngineLogo,
	"Returns one IGDB GameEngineLogo by id.",
	"Looks up many IGDB GameEngineLogo records by id in one request."
);

igdb_entity_routes!(
	"/igdb/game-localization",
	get_igdb_game_localization_by_id,
	"/igdb/game-localizations",
	get_igdb_game_localizations_by_ids,
	get_game_localization_by_id_cached,
	GameLocalization,
	"Returns one IGDB GameLocalization by id.",
	"Looks up many IGDB GameLocalization records by id in one request."
);

igdb_entity_routes!(
	"/igdb/game-mode",
	get_igdb_game_mode_by_id,
	"/igdb/game-modes",
	get_igdb_game_modes_by_ids,
	get_game_mode_by_id_cached,
	GameMode,
	"Returns one IGDB GameMode by id.",
	"Looks up many IGDB GameMode records by id in one request."
);

igdb_entity_routes!(
	"/igdb/game-version",
	get_igdb_game_version_by_id,
	"/igdb/game-versions",
	get_igdb_game_versions_by_ids,
	get_game_version_by_id_cached,
	GameVersion,
	"Returns one IGDB GameVersion by id.",
	"Looks up many IGDB GameVersion records by id in one request."
);

igdb_entity_routes!(
	"/igdb/game-version-feature",
	get_igdb_game_version_feature_by_id,
	"/igdb/game-version-features",
	get_igdb_game_version_features_by_ids,
	get_game_version_feature_by_id_cached,
	GameVersionFeature,
	"Returns one IGDB GameVersionFeature by id.",
	"Looks up many IGDB GameVersionFeature records by id in one request."
);

igdb_entity_routes!(
	"/igdb/game-version-feature-value",
	get_igdb_game_version_feature_value_by_id,
	"/igdb/game-version-feature-values",
	get_igdb_game_version_feature_values_by_ids,
	get_game_version_feature_value_by_id_cached,
	GameVersionFeatureValue,
	"Returns one IGDB GameVersionFeatureValue by id.",
	"Looks up many IGDB GameVersionFeatureValue records by id in one request."
);

igdb_entity_routes!(
	"/igdb/game-video",
	get_igdb_game_video_by_id,
	"/igdb/game-videos",
	get_igdb_game_videos_by_ids,
	get_game_video_by_id_cached,
	GameVideo,
	"Returns one IGDB GameVideo by id.",
	"Looks up many IGDB GameVideo records by id in one request."
);

igdb_entity_routes!(
	"/igdb/involved-company",
	get_igdb_involved_company_by_id,
	"/igdb/involved-companies",
	get_igdb_involved_companies_by_ids,
	get_involved_company_by_id_cached,
	InvolvedCompany,
	"Returns one IGDB InvolvedCompany by id.",
	"Looks up many IGDB InvolvedCompany records by id in one request."
);

igdb_entity_routes!(
	"/igdb/keyword",
	get_igdb_keyword_by_id,
	"/igdb/keywords",
	get_igdb_keywords_by_ids,
	get_keyword_by_id_cached,
	Keyword,
	"Returns one IGDB Keyword by id.",
	"Looks up many IGDB Keyword records by id in one request."
);

igdb_entity_routes!(
	"/igdb/language",
	get_igdb_language_by_id,
	"/igdb/languages",
	get_igdb_languages_by_ids,
	get_language_by_id_cached,
	Language,
	"Returns one IGDB Language by id.",
	"Looks up many IGDB Language records by id in one request."
);

igdb_entity_routes!(
	"/igdb/language-support",
	get_igdb_language_support_by_id,
	"/igdb/language-supports",
	get_igdb_language_supports_by_ids,
	get_language_support_by_id_cached,
	LanguageSupport,
	"Returns one IGDB LanguageSupport by id.",
	"Looks up many IGDB LanguageSupport records by id in one request."
);

igdb_entity_routes!(
	"/igdb/language-support-type",
	get_igdb_language_support_type_by_id,
	"/igdb/language-support-types",
	get_igdb_language_support_types_by_ids,
	get_language_support_type_by_id_cached,
	LanguageSupportType,
	"Returns one IGDB LanguageSupportType by id.",
	"Looks up many IGDB LanguageSupportType records by id in one request."
);

igdb_entity_routes!(
	"/igdb/multiplayer-mode",
	get_igdb_multiplayer_mode_by_id,
	"/igdb/multiplayer-modes",
	get_igdb_multiplayer_modes_by_ids,
	get_multiplayer_mode_by_id_cached,
	MultiplayerMode,
	"Returns one IGDB MultiplayerMode by id.",
	"Looks up many IGDB MultiplayerMode records by id in one request."
);

igdb_entity_routes!(
	"/igdb/network-type",
	get_igdb_network_type_by_id,
	"/igdb/network-types",
	get_igdb_network_types_by_ids,
	get_network_type_by_id_cached,
	NetworkType,
	"Returns one IGDB NetworkType by id.",
	"Looks up many IGDB NetworkType records by id in one request."
);

igdb_entity_routes!(
	"/igdb/platform",
	get_igdb_platform_by_id,
	"/igdb/platforms",
	get_igdb_platforms_by_ids,
	get_platform_by_id_cached,
	Platform,
	"Returns one IGDB Platform by id.",
	"Looks up many IGDB Platform records by id in one request."
);

igdb_entity_routes!(
	"/igdb/platform-family",
	get_igdb_platform_family_by_id,
	"/igdb/platform-families",
	get_igdb_platform_families_by_ids,
	get_platform_family_by_id_cached,
	PlatformFamily,
	"Returns one IGDB PlatformFamily by id.",
	"Looks up many IGDB PlatformFamily records by id in one request."
);

igdb_entity_routes!(
	"/igdb/platform-logo",
	get_igdb_platform_logo_by_id,
	"/igdb/platform-logos",
	get_igdb_platform_logos_by_ids,
	get_platform_logo_by_id_cached,
	PlatformLogo,
	"Returns one IGDB PlatformLogo by id.",
	"Looks up many IGDB PlatformLogo records by id in one request."
);

igdb_entity_routes!(
	"/igdb/platform-version",
	get_igdb_platform_version_by_id,
	"/igdb/platform-versions",
	get_igdb_platform_versions_by_ids,
	get_platform_version_by_id_cached,
	PlatformVersion,
	"Returns one IGDB PlatformVersion by id.",
	"Looks up many IGDB PlatformVersion records by id in one request."
);

igdb_entity_routes!(
	"/igdb/platform-version-company",
	get_igdb_platform_version_company_by_id,
	"/igdb/platform-version-companies",
	get_igdb_platform_version_companies_by_ids,
	get_platform_version_company_by_id_cached,
	PlatformVersionCompany,
	"Returns one IGDB PlatformVersionCompany by id.",
	"Looks up many IGDB PlatformVersionCompany records by id in one request."
);

igdb_entity_routes!(
	"/igdb/platform-version-release-date",
	get_igdb_platform_version_release_date_by_id,
	"/igdb/platform-version-release-dates",
	get_igdb_platform_version_release_dates_by_ids,
	get_platform_version_release_date_by_id_cached,
	PlatformVersionReleaseDate,
	"Returns one IGDB PlatformVersionReleaseDate by id.",
	"Looks up many IGDB PlatformVersionReleaseDate records by id in one request."
);

igdb_entity_routes!(
	"/igdb/platform-website",
	get_igdb_platform_website_by_id,
	"/igdb/platform-websites",
	get_igdb_platform_websites_by_ids,
	get_platform_website_by_id_cached,
	PlatformWebsite,
	"Returns one IGDB PlatformWebsite by id.",
	"Looks up many IGDB PlatformWebsite records by id in one request."
);

igdb_entity_routes!(
	"/igdb/player-perspective",
	get_igdb_player_perspective_by_id,
	"/igdb/player-perspectives",
	get_igdb_player_perspectives_by_ids,
	get_player_perspective_by_id_cached,
	PlayerPerspective,
	"Returns one IGDB PlayerPerspective by id.",
	"Looks up many IGDB PlayerPerspective records by id in one request."
);

igdb_entity_routes!(
	"/igdb/popularity-primitive",
	get_igdb_popularity_primitive_by_id,
	"/igdb/popularity-primitives",
	get_igdb_popularity_primitives_by_ids,
	get_popularity_primitive_by_id_cached,
	PopularityPrimitive,
	"Returns one IGDB PopularityPrimitive by id.",
	"Looks up many IGDB PopularityPrimitive records by id in one request."
);

igdb_entity_routes!(
	"/igdb/popularity-type",
	get_igdb_popularity_type_by_id,
	"/igdb/popularity-types",
	get_igdb_popularity_types_by_ids,
	get_popularity_type_by_id_cached,
	PopularityType,
	"Returns one IGDB PopularityType by id.",
	"Looks up many IGDB PopularityType records by id in one request."
);

igdb_entity_routes!(
	"/igdb/region",
	get_igdb_region_by_id,
	"/igdb/regions",
	get_igdb_regions_by_ids,
	get_region_by_id_cached,
	Region,
	"Returns one IGDB Region by id.",
	"Looks up many IGDB Region records by id in one request."
);

igdb_entity_routes!(
	"/igdb/release-date",
	get_igdb_release_date_by_id,
	"/igdb/release-dates",
	get_igdb_release_dates_by_ids,
	get_release_date_by_id_cached,
	ReleaseDate,
	"Returns one IGDB ReleaseDate by id.",
	"Looks up many IGDB ReleaseDate records by id in one request."
);

igdb_entity_routes!(
	"/igdb/release-date-status",
	get_igdb_release_date_status_by_id,
	"/igdb/release-date-statuses",
	get_igdb_release_date_statuses_by_ids,
	get_release_date_status_by_id_cached,
	ReleaseDateStatus,
	"Returns one IGDB ReleaseDateStatus by id.",
	"Looks up many IGDB ReleaseDateStatus records by id in one request."
);

igdb_entity_routes!(
	"/igdb/screenshot",
	get_igdb_screenshot_by_id,
	"/igdb/screenshots",
	get_igdb_screenshots_by_ids,
	get_screenshot_by_id_cached,
	Screenshot,
	"Returns one IGDB Screenshot by id.",
	"Looks up many IGDB Screenshot records by id in one request."
);

igdb_entity_routes!(
	"/igdb/theme",
	get_igdb_theme_by_id,
	"/igdb/themes",
	get_igdb_themes_by_ids,
	get_theme_by_id_cached,
	Theme,
	"Returns one IGDB Theme by id.",
	"Looks up many IGDB Theme records by id in one request."
);

igdb_entity_routes!(
	"/igdb/website",
	get_igdb_website_by_id,
	"/igdb/websites",
	get_igdb_websites_by_ids,
	get_website_by_id_cached,
	Website,
	"Returns one IGDB Website by id.",
	"Looks up many IGDB Website records by id in one request."
);
