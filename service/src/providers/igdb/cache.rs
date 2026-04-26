use crate::providers::igdb::IgdbClient;
use crate::providers::igdb::model::{
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
use redis::AsyncTypedCommands;
use std::time::Duration;

const IGDB_CACHE_LIFETIME: u64 = Duration::from_secs(60 * 60 * 24).as_secs(); // 1 day

const L1_MAX_CAPACITY: u64 = 10_000;
const L1_TIME_TO_IDLE: Duration = Duration::from_secs(15 * 60);

/// IGDB-prefilled thin wrapper around `$crate::__cached_lookup_impl`. Keeps the
/// per-entity invocations below terse; the shared macro body lives in
/// `service/src/cache/macros.rs`.
macro_rules! cached_lookup {
	($fn_name:ident, $ty:ty, $fetch:ident, $segment:literal, $label:literal) => {
		$crate::__cached_lookup_impl!(
			$fn_name,
			&IgdbClient,
			"igdb",
			$segment,
			IGDB_CACHE_LIFETIME,
			$ty,
			$fetch,
			$label
		);
	};
}

/// IGDB-prefilled thin wrapper around `$crate::__cached_reference_lookup_impl`.
/// Adds a per-worker in-process L1 (moka) in front of Redis for small, hot
/// reference entities.
macro_rules! cached_reference_lookup {
	($fn_name:ident, $ty:ty, $fetch:ident, $segment:literal, $label:literal) => {
		$crate::__cached_reference_lookup_impl!(
			$fn_name,
			&IgdbClient,
			"igdb",
			$segment,
			"igdb-l1",
			L1_MAX_CAPACITY,
			L1_TIME_TO_IDLE,
			IGDB_CACHE_LIFETIME,
			$ty,
			$fetch,
			$label
		);
	};
}

macro_rules! cached_lookup_by_slug {
	($fn_name:ident, $ty:ty, $fetch:ident, $segment:literal, $label:literal) => {
		$crate::__cached_lookup_by_slug_impl!(
			$fn_name,
			&IgdbClient,
			"igdb",
			$segment,
			IGDB_CACHE_LIFETIME,
			$ty,
			$fetch,
			$label
		);
	};
}

macro_rules! cached_search {
	($fn_name:ident, $ty:ty, $fetch:ident, $segment:literal, $label:literal) => {
		$crate::__cached_search_impl!(
			$fn_name,
			&IgdbClient,
			"igdb",
			$segment,
			IGDB_CACHE_LIFETIME,
			$ty,
			$fetch,
			$label
		);
	};
}

cached_lookup!(
	get_age_rating_by_id_cached,
	AgeRating,
	get_age_rating_by_id,
	"age_rating",
	"Age Rating"
);
cached_reference_lookup!(
	get_age_rating_category_by_id_cached,
	AgeRatingCategory,
	get_age_rating_category_by_id,
	"age_rating_category",
	"Age Rating Category"
);
cached_lookup!(
	get_age_rating_content_description_type_by_id_cached,
	AgeRatingContentDescriptionType,
	get_age_rating_content_description_type_by_id,
	"age_rating_content_description_type",
	"Age Rating Content Description Type"
);
cached_lookup!(
	get_age_rating_content_description_v2_by_id_cached,
	AgeRatingContentDescriptionV2,
	get_age_rating_content_description_v2_by_id,
	"age_rating_content_description_v2",
	"Age Rating Content Description V2"
);
cached_lookup!(
	get_age_rating_organization_by_id_cached,
	AgeRatingOrganization,
	get_age_rating_organization_by_id,
	"age_rating_organization",
	"Age Rating Organization"
);
cached_lookup!(
	get_alternative_name_by_id_cached,
	AlternativeName,
	get_alternative_name_by_id,
	"alternative_name",
	"Alternative Name"
);
cached_lookup!(
	get_artwork_by_id_cached,
	Artwork,
	get_artwork_by_id,
	"artwork",
	"Artwork"
);
cached_lookup!(
	get_artwork_type_by_id_cached,
	ArtworkType,
	get_artwork_type_by_id,
	"artwork_type",
	"Artwork Type"
);
cached_lookup!(
	get_character_mug_shot_by_id_cached,
	CharacterMugShot,
	get_character_mug_shot_by_id,
	"character_mug_shot",
	"Character Mug Shot"
);
cached_lookup!(
	get_collection_by_id_cached,
	Collection,
	get_collection_by_id,
	"collection",
	"Collection"
);
cached_lookup!(
	get_company_size_by_id_cached,
	CompanySize,
	get_company_size_by_id,
	"company_size",
	"Company Size"
);
cached_reference_lookup!(
	get_company_status_by_id_cached,
	CompanyStatus,
	get_company_status_by_id,
	"company_status",
	"Company Status"
);
cached_lookup!(
	get_company_type_by_id_cached,
	CompanyType,
	get_company_type_by_id,
	"company_type",
	"Company Type"
);
cached_lookup!(
	get_company_type_history_by_id_cached,
	CompanyTypeHistory,
	get_company_type_history_by_id,
	"company_type_history",
	"Company Type History"
);
cached_lookup!(
	get_cover_by_id_cached,
	Cover,
	get_cover_by_id,
	"cover",
	"Cover"
);
cached_reference_lookup!(
	get_date_format_by_id_cached,
	DateFormat,
	get_date_format_by_id,
	"date_format",
	"Date Format"
);
cached_lookup!(
	get_entity_type_by_id_cached,
	EntityType,
	get_entity_type_by_id,
	"entity_type",
	"Entity Type"
);
cached_lookup!(
	get_external_game_by_id_cached,
	ExternalGame,
	get_external_game_by_id,
	"external_game",
	"External Game"
);
cached_lookup!(
	get_external_game_source_by_id_cached,
	ExternalGameSource,
	get_external_game_source_by_id,
	"external_game_source",
	"External Game Source"
);
cached_lookup!(
	get_franchise_by_id_cached,
	Franchise,
	get_franchise_by_id,
	"franchise",
	"Franchise"
);
cached_lookup!(
	get_game_by_id_cached,
	Game,
	get_game_by_id,
	"game",
	"Game"
);
cached_lookup_by_slug!(
	get_game_by_slug_cached,
	Game,
	get_game_by_slug,
	"game:slug",
	"Game"
);
cached_search!(
	search_game_by_name_cached,
	Game,
	search_game_by_name,
	"game:search",
	"Search Game by Name"
);
cached_lookup!(
	get_game_release_format_by_id_cached,
	GameReleaseFormat,
	get_game_release_format_by_id,
	"game_release_format",
	"Game Release Format"
);
cached_reference_lookup!(
	get_game_status_by_id_cached,
	GameStatus,
	get_game_status_by_id,
	"game_status",
	"Game Status"
);
cached_lookup!(
	get_game_time_to_beat_by_id_cached,
	GameTimeToBeat,
	get_game_time_to_beat_by_id,
	"game_time_to_beat",
	"Game Time To Beat"
);
cached_reference_lookup!(
	get_game_type_by_id_cached,
	GameType,
	get_game_type_by_id,
	"game_type",
	"Game Type"
);
cached_reference_lookup!(
	get_genre_by_id_cached,
	Genre,
	get_genre_by_id,
	"genre",
	"Genre"
);
cached_reference_lookup!(
	get_platform_type_by_id_cached,
	PlatformType,
	get_platform_type_by_id,
	"platform_type",
	"Platform Type"
);
cached_lookup!(
	get_release_date_region_by_id_cached,
	ReleaseDateRegion,
	get_release_date_region_by_id,
	"release_date_region",
	"Release Date Region"
);
cached_lookup!(
	get_report_by_id_cached,
	Report,
	get_report_by_id,
	"report",
	"Report"
);
cached_lookup!(
	get_report_type_by_id_cached,
	ReportType,
	get_report_type_by_id,
	"report_type",
	"Report Type"
);
cached_lookup!(
	get_website_type_by_id_cached,
	WebsiteType,
	get_website_type_by_id,
	"website_type",
	"Website Type"
);
cached_lookup!(
	get_character_by_id_cached,
	Character,
	get_character_by_id,
	"character",
	"Character"
);
cached_lookup!(
	get_character_gender_by_id_cached,
	CharacterGender,
	get_character_gender_by_id,
	"character_gender",
	"Character Gender"
);
cached_lookup!(
	get_character_species_by_id_cached,
	CharacterSpecies,
	get_character_species_by_id,
	"character_species",
	"Character Species"
);
cached_lookup!(
	get_collection_membership_by_id_cached,
	CollectionMembership,
	get_collection_membership_by_id,
	"collection_membership",
	"Collection Membership"
);
cached_lookup!(
	get_collection_membership_type_by_id_cached,
	CollectionMembershipType,
	get_collection_membership_type_by_id,
	"collection_membership_type",
	"Collection Membership Type"
);
cached_lookup!(
	get_collection_relation_by_id_cached,
	CollectionRelation,
	get_collection_relation_by_id,
	"collection_relation",
	"Collection Relation"
);
cached_lookup!(
	get_collection_relation_type_by_id_cached,
	CollectionRelationType,
	get_collection_relation_type_by_id,
	"collection_relation_type",
	"Collection Relation Type"
);
cached_lookup!(
	get_collection_type_by_id_cached,
	CollectionType,
	get_collection_type_by_id,
	"collection_type",
	"Collection Type"
);
cached_lookup!(
	get_company_by_id_cached,
	Company,
	get_company_by_id,
	"company",
	"Company"
);
cached_lookup!(
	get_company_logo_by_id_cached,
	CompanyLogo,
	get_company_logo_by_id,
	"company_logo",
	"Company Logo"
);
cached_lookup!(
	get_company_website_by_id_cached,
	CompanyWebsite,
	get_company_website_by_id,
	"company_website",
	"Company Website"
);
cached_lookup!(
	get_event_by_id_cached,
	Event,
	get_event_by_id,
	"event",
	"Event"
);
cached_lookup!(
	get_event_logo_by_id_cached,
	EventLogo,
	get_event_logo_by_id,
	"event_logo",
	"Event Logo"
);
cached_lookup!(
	get_event_network_by_id_cached,
	EventNetwork,
	get_event_network_by_id,
	"event_network",
	"Event Network"
);
cached_lookup!(
	get_game_engine_by_id_cached,
	GameEngine,
	get_game_engine_by_id,
	"game_engine",
	"Game Engine"
);
cached_lookup!(
	get_game_engine_logo_by_id_cached,
	GameEngineLogo,
	get_game_engine_logo_by_id,
	"game_engine_logo",
	"Game Engine Logo"
);
cached_lookup!(
	get_game_localization_by_id_cached,
	GameLocalization,
	get_game_localization_by_id,
	"game_localization",
	"Game Localization"
);
cached_reference_lookup!(
	get_game_mode_by_id_cached,
	GameMode,
	get_game_mode_by_id,
	"game_mode",
	"Game Mode"
);
cached_lookup!(
	get_game_version_by_id_cached,
	GameVersion,
	get_game_version_by_id,
	"game_version",
	"Game Version"
);
cached_lookup!(
	get_game_version_feature_by_id_cached,
	GameVersionFeature,
	get_game_version_feature_by_id,
	"game_version_feature",
	"Game Version Feature"
);
cached_lookup!(
	get_game_version_feature_value_by_id_cached,
	GameVersionFeatureValue,
	get_game_version_feature_value_by_id,
	"game_version_feature_value",
	"Game Version Feature Value"
);
cached_lookup!(
	get_game_video_by_id_cached,
	GameVideo,
	get_game_video_by_id,
	"game_video",
	"Game Video"
);
cached_lookup!(
	get_involved_company_by_id_cached,
	InvolvedCompany,
	get_involved_company_by_id,
	"involved_company",
	"Involved Company"
);
cached_lookup!(
	get_keyword_by_id_cached,
	Keyword,
	get_keyword_by_id,
	"keyword",
	"Keyword"
);
cached_reference_lookup!(
	get_language_by_id_cached,
	Language,
	get_language_by_id,
	"language",
	"Language"
);
cached_lookup!(
	get_language_support_by_id_cached,
	LanguageSupport,
	get_language_support_by_id,
	"language_support",
	"Language Support"
);
cached_lookup!(
	get_language_support_type_by_id_cached,
	LanguageSupportType,
	get_language_support_type_by_id,
	"language_support_type",
	"Language Support Type"
);
cached_lookup!(
	get_multiplayer_mode_by_id_cached,
	MultiplayerMode,
	get_multiplayer_mode_by_id,
	"multiplayer_mode",
	"Multiplayer Mode"
);
cached_reference_lookup!(
	get_network_type_by_id_cached,
	NetworkType,
	get_network_type_by_id,
	"network_type",
	"Network Type"
);
cached_lookup!(
	get_platform_by_id_cached,
	Platform,
	get_platform_by_id,
	"platform",
	"Platform"
);
cached_lookup!(
	get_platform_family_by_id_cached,
	PlatformFamily,
	get_platform_family_by_id,
	"platform_family",
	"Platform Family"
);
cached_lookup!(
	get_platform_logo_by_id_cached,
	PlatformLogo,
	get_platform_logo_by_id,
	"platform_logo",
	"Platform Logo"
);
cached_lookup!(
	get_platform_version_by_id_cached,
	PlatformVersion,
	get_platform_version_by_id,
	"platform_version",
	"Platform Version"
);
cached_lookup!(
	get_platform_version_company_by_id_cached,
	PlatformVersionCompany,
	get_platform_version_company_by_id,
	"platform_version_company",
	"Platform Version Company"
);
cached_lookup!(
	get_platform_version_release_date_by_id_cached,
	PlatformVersionReleaseDate,
	get_platform_version_release_date_by_id,
	"platform_version_release_date",
	"Platform Version Release Date"
);
cached_lookup!(
	get_platform_website_by_id_cached,
	PlatformWebsite,
	get_platform_website_by_id,
	"platform_website",
	"Platform Website"
);
cached_reference_lookup!(
	get_player_perspective_by_id_cached,
	PlayerPerspective,
	get_player_perspective_by_id,
	"player_perspective",
	"Player Perspective"
);
cached_lookup!(
	get_popularity_primitive_by_id_cached,
	PopularityPrimitive,
	get_popularity_primitive_by_id,
	"popularity_primitive",
	"Popularity Primitive"
);
cached_reference_lookup!(
	get_popularity_type_by_id_cached,
	PopularityType,
	get_popularity_type_by_id,
	"popularity_type",
	"Popularity Type"
);
cached_reference_lookup!(
	get_region_by_id_cached,
	Region,
	get_region_by_id,
	"region",
	"Region"
);
cached_lookup!(
	get_release_date_by_id_cached,
	ReleaseDate,
	get_release_date_by_id,
	"release_date",
	"Release Date"
);
cached_lookup!(
	get_release_date_status_by_id_cached,
	ReleaseDateStatus,
	get_release_date_status_by_id,
	"release_date_status",
	"Release Date Status"
);
cached_lookup!(
	get_screenshot_by_id_cached,
	Screenshot,
	get_screenshot_by_id,
	"screenshot",
	"Screenshot"
);
cached_reference_lookup!(
	get_theme_by_id_cached,
	Theme,
	get_theme_by_id,
	"theme",
	"Theme"
);
cached_lookup!(
	get_website_by_id_cached,
	Website,
	get_website_by_id,
	"website",
	"Website"
);
