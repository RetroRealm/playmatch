use crate::cache::{CACHE_KEY_VERSION, CACHE_PREFIX, CacheKey};
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
	($fn_name:ident, $ty:ty, $fetch:ident, $variant:ident, $label:literal) => {
		$crate::__cached_lookup_impl!(
			$fn_name,
			&IgdbClient,
			IgdbCacheType,
			"igdb",
			IGDB_CACHE_LIFETIME,
			$ty,
			$fetch,
			$variant,
			$label
		);
	};
}

/// IGDB-prefilled thin wrapper around `$crate::__cached_reference_lookup_impl`.
/// Adds a per-worker in-process L1 (moka) in front of Redis for small, hot
/// reference entities.
macro_rules! cached_reference_lookup {
	($fn_name:ident, $ty:ty, $fetch:ident, $variant:ident, $label:literal) => {
		$crate::__cached_reference_lookup_impl!(
			$fn_name,
			&IgdbClient,
			IgdbCacheType,
			"igdb",
			"igdb-l1",
			L1_MAX_CAPACITY,
			L1_TIME_TO_IDLE,
			IGDB_CACHE_LIFETIME,
			$ty,
			$fetch,
			$variant,
			$label
		);
	};
}

macro_rules! cached_lookup_by_slug {
	($fn_name:ident, $ty:ty, $fetch:ident, $variant:ident, $label:literal) => {
		$crate::__cached_lookup_by_slug_impl!(
			$fn_name,
			&IgdbClient,
			IgdbCacheType,
			"igdb",
			IGDB_CACHE_LIFETIME,
			$ty,
			$fetch,
			$variant,
			$label
		);
	};
}

macro_rules! cached_search {
	($fn_name:ident, $ty:ty, $fetch:ident, $variant:ident, $label:literal) => {
		$crate::__cached_search_impl!(
			$fn_name,
			&IgdbClient,
			IgdbCacheType,
			"igdb",
			IGDB_CACHE_LIFETIME,
			$ty,
			$fetch,
			$variant,
			$label
		);
	};
}

cached_lookup!(
	get_age_rating_by_id_cached,
	AgeRating,
	get_age_rating_by_id,
	GetAgeRatingById,
	"Age Rating"
);
cached_reference_lookup!(
	get_age_rating_category_by_id_cached,
	AgeRatingCategory,
	get_age_rating_category_by_id,
	GetAgeRatingCategoryById,
	"Age Rating Category"
);
cached_lookup!(
	get_age_rating_content_description_type_by_id_cached,
	AgeRatingContentDescriptionType,
	get_age_rating_content_description_type_by_id,
	GetAgeRatingContentDescriptionTypeById,
	"Age Rating Content Description Type"
);
cached_lookup!(
	get_age_rating_content_description_v2_by_id_cached,
	AgeRatingContentDescriptionV2,
	get_age_rating_content_description_v2_by_id,
	GetAgeRatingContentDescriptionV2ById,
	"Age Rating Content Description V2"
);
cached_lookup!(
	get_age_rating_organization_by_id_cached,
	AgeRatingOrganization,
	get_age_rating_organization_by_id,
	GetAgeRatingOrganizationById,
	"Age Rating Organization"
);
cached_lookup!(
	get_alternative_name_by_id_cached,
	AlternativeName,
	get_alternative_name_by_id,
	GetAlternativeNameById,
	"Alternative Name"
);
cached_lookup!(
	get_artwork_by_id_cached,
	Artwork,
	get_artwork_by_id,
	GetArtworkById,
	"Artwork"
);
cached_lookup!(
	get_artwork_type_by_id_cached,
	ArtworkType,
	get_artwork_type_by_id,
	GetArtworkTypeById,
	"Artwork Type"
);
cached_lookup!(
	get_character_mug_shot_by_id_cached,
	CharacterMugShot,
	get_character_mug_shot_by_id,
	GetCharacterMugShotById,
	"Character Mug Shot"
);
cached_lookup!(
	get_collection_by_id_cached,
	Collection,
	get_collection_by_id,
	GetCollectionById,
	"Collection"
);
cached_lookup!(
	get_company_size_by_id_cached,
	CompanySize,
	get_company_size_by_id,
	GetCompanySizeById,
	"Company Size"
);
cached_reference_lookup!(
	get_company_status_by_id_cached,
	CompanyStatus,
	get_company_status_by_id,
	GetCompanyStatusById,
	"Company Status"
);
cached_lookup!(
	get_company_type_by_id_cached,
	CompanyType,
	get_company_type_by_id,
	GetCompanyTypeById,
	"Company Type"
);
cached_lookup!(
	get_company_type_history_by_id_cached,
	CompanyTypeHistory,
	get_company_type_history_by_id,
	GetCompanyTypeHistoryById,
	"Company Type History"
);
cached_lookup!(
	get_cover_by_id_cached,
	Cover,
	get_cover_by_id,
	GetCoverById,
	"Cover"
);
cached_reference_lookup!(
	get_date_format_by_id_cached,
	DateFormat,
	get_date_format_by_id,
	GetDateFormatById,
	"Date Format"
);
cached_lookup!(
	get_entity_type_by_id_cached,
	EntityType,
	get_entity_type_by_id,
	GetEntityTypeById,
	"Entity Type"
);
cached_lookup!(
	get_external_game_by_id_cached,
	ExternalGame,
	get_external_game_by_id,
	GetExternalGameById,
	"External Game"
);
cached_lookup!(
	get_external_game_source_by_id_cached,
	ExternalGameSource,
	get_external_game_source_by_id,
	GetExternalGameSourceById,
	"External Game Source"
);
cached_lookup!(
	get_franchise_by_id_cached,
	Franchise,
	get_franchise_by_id,
	GetFranchiseById,
	"Franchise"
);
cached_lookup!(
	get_game_by_id_cached,
	Game,
	get_game_by_id,
	GetGameById,
	"Game"
);
cached_lookup_by_slug!(
	get_game_by_slug_cached,
	Game,
	get_game_by_slug,
	GetGameBySlug,
	"Game"
);
cached_search!(
	search_game_by_name_cached,
	Game,
	search_game_by_name,
	SearchGameByName,
	"Search Game by Name"
);
cached_lookup!(
	get_game_release_format_by_id_cached,
	GameReleaseFormat,
	get_game_release_format_by_id,
	GetGameReleaseFormatById,
	"Game Release Format"
);
cached_reference_lookup!(
	get_game_status_by_id_cached,
	GameStatus,
	get_game_status_by_id,
	GetGameStatusById,
	"Game Status"
);
cached_lookup!(
	get_game_time_to_beat_by_id_cached,
	GameTimeToBeat,
	get_game_time_to_beat_by_id,
	GetGameTimeToBeatById,
	"Game Time To Beat"
);
cached_reference_lookup!(
	get_game_type_by_id_cached,
	GameType,
	get_game_type_by_id,
	GetGameTypeById,
	"Game Type"
);
cached_reference_lookup!(
	get_genre_by_id_cached,
	Genre,
	get_genre_by_id,
	GetGenreById,
	"Genre"
);
cached_reference_lookup!(
	get_platform_type_by_id_cached,
	PlatformType,
	get_platform_type_by_id,
	GetPlatformTypeById,
	"Platform Type"
);
cached_lookup!(
	get_release_date_region_by_id_cached,
	ReleaseDateRegion,
	get_release_date_region_by_id,
	GetReleaseDateRegionById,
	"Release Date Region"
);
cached_lookup!(
	get_report_by_id_cached,
	Report,
	get_report_by_id,
	GetReportById,
	"Report"
);
cached_lookup!(
	get_report_type_by_id_cached,
	ReportType,
	get_report_type_by_id,
	GetReportTypeById,
	"Report Type"
);
cached_lookup!(
	get_website_type_by_id_cached,
	WebsiteType,
	get_website_type_by_id,
	GetWebsiteTypeById,
	"Website Type"
);
cached_lookup!(
	get_character_by_id_cached,
	Character,
	get_character_by_id,
	GetCharacterById,
	"Character"
);
cached_lookup!(
	get_character_gender_by_id_cached,
	CharacterGender,
	get_character_gender_by_id,
	GetCharacterGenderById,
	"Character Gender"
);
cached_lookup!(
	get_character_species_by_id_cached,
	CharacterSpecies,
	get_character_species_by_id,
	GetCharacterSpeciesById,
	"Character Species"
);
cached_lookup!(
	get_collection_membership_by_id_cached,
	CollectionMembership,
	get_collection_membership_by_id,
	GetCollectionMembershipById,
	"Collection Membership"
);
cached_lookup!(
	get_collection_membership_type_by_id_cached,
	CollectionMembershipType,
	get_collection_membership_type_by_id,
	GetCollectionMembershipTypeById,
	"Collection Membership Type"
);
cached_lookup!(
	get_collection_relation_by_id_cached,
	CollectionRelation,
	get_collection_relation_by_id,
	GetCollectionRelationById,
	"Collection Relation"
);
cached_lookup!(
	get_collection_relation_type_by_id_cached,
	CollectionRelationType,
	get_collection_relation_type_by_id,
	GetCollectionRelationTypeById,
	"Collection Relation Type"
);
cached_lookup!(
	get_collection_type_by_id_cached,
	CollectionType,
	get_collection_type_by_id,
	GetCollectionTypeById,
	"Collection Type"
);
cached_lookup!(
	get_company_by_id_cached,
	Company,
	get_company_by_id,
	GetCompanyById,
	"Company"
);
cached_lookup!(
	get_company_logo_by_id_cached,
	CompanyLogo,
	get_company_logo_by_id,
	GetCompanyLogoById,
	"Company Logo"
);
cached_lookup!(
	get_company_website_by_id_cached,
	CompanyWebsite,
	get_company_website_by_id,
	GetCompanyWebsiteById,
	"Company Website"
);
cached_lookup!(
	get_event_by_id_cached,
	Event,
	get_event_by_id,
	GetEventById,
	"Event"
);
cached_lookup!(
	get_event_logo_by_id_cached,
	EventLogo,
	get_event_logo_by_id,
	GetEventLogoById,
	"Event Logo"
);
cached_lookup!(
	get_event_network_by_id_cached,
	EventNetwork,
	get_event_network_by_id,
	GetEventNetworkById,
	"Event Network"
);
cached_lookup!(
	get_game_engine_by_id_cached,
	GameEngine,
	get_game_engine_by_id,
	GetGameEngineById,
	"Game Engine"
);
cached_lookup!(
	get_game_engine_logo_by_id_cached,
	GameEngineLogo,
	get_game_engine_logo_by_id,
	GetGameEngineLogoById,
	"Game Engine Logo"
);
cached_lookup!(
	get_game_localization_by_id_cached,
	GameLocalization,
	get_game_localization_by_id,
	GetGameLocalizationById,
	"Game Localization"
);
cached_reference_lookup!(
	get_game_mode_by_id_cached,
	GameMode,
	get_game_mode_by_id,
	GetGameModeById,
	"Game Mode"
);
cached_lookup!(
	get_game_version_by_id_cached,
	GameVersion,
	get_game_version_by_id,
	GetGameVersionById,
	"Game Version"
);
cached_lookup!(
	get_game_version_feature_by_id_cached,
	GameVersionFeature,
	get_game_version_feature_by_id,
	GetGameVersionFeatureById,
	"Game Version Feature"
);
cached_lookup!(
	get_game_version_feature_value_by_id_cached,
	GameVersionFeatureValue,
	get_game_version_feature_value_by_id,
	GetGameVersionFeatureValueById,
	"Game Version Feature Value"
);
cached_lookup!(
	get_game_video_by_id_cached,
	GameVideo,
	get_game_video_by_id,
	GetGameVideoById,
	"Game Video"
);
cached_lookup!(
	get_involved_company_by_id_cached,
	InvolvedCompany,
	get_involved_company_by_id,
	GetInvolvedCompanyById,
	"Involved Company"
);
cached_lookup!(
	get_keyword_by_id_cached,
	Keyword,
	get_keyword_by_id,
	GetKeywordById,
	"Keyword"
);
cached_reference_lookup!(
	get_language_by_id_cached,
	Language,
	get_language_by_id,
	GetLanguageById,
	"Language"
);
cached_lookup!(
	get_language_support_by_id_cached,
	LanguageSupport,
	get_language_support_by_id,
	GetLanguageSupportById,
	"Language Support"
);
cached_lookup!(
	get_language_support_type_by_id_cached,
	LanguageSupportType,
	get_language_support_type_by_id,
	GetLanguageSupportTypeById,
	"Language Support Type"
);
cached_lookup!(
	get_multiplayer_mode_by_id_cached,
	MultiplayerMode,
	get_multiplayer_mode_by_id,
	GetMultiplayerModeById,
	"Multiplayer Mode"
);
cached_reference_lookup!(
	get_network_type_by_id_cached,
	NetworkType,
	get_network_type_by_id,
	GetNetworkTypeById,
	"Network Type"
);
cached_lookup!(
	get_platform_by_id_cached,
	Platform,
	get_platform_by_id,
	GetPlatformById,
	"Platform"
);
cached_lookup!(
	get_platform_family_by_id_cached,
	PlatformFamily,
	get_platform_family_by_id,
	GetPlatformFamilyById,
	"Platform Family"
);
cached_lookup!(
	get_platform_logo_by_id_cached,
	PlatformLogo,
	get_platform_logo_by_id,
	GetPlatformLogoById,
	"Platform Logo"
);
cached_lookup!(
	get_platform_version_by_id_cached,
	PlatformVersion,
	get_platform_version_by_id,
	GetPlatformVersionById,
	"Platform Version"
);
cached_lookup!(
	get_platform_version_company_by_id_cached,
	PlatformVersionCompany,
	get_platform_version_company_by_id,
	GetPlatformVersionCompanyById,
	"Platform Version Company"
);
cached_lookup!(
	get_platform_version_release_date_by_id_cached,
	PlatformVersionReleaseDate,
	get_platform_version_release_date_by_id,
	GetPlatformVersionReleaseDateById,
	"Platform Version Release Date"
);
cached_lookup!(
	get_platform_website_by_id_cached,
	PlatformWebsite,
	get_platform_website_by_id,
	GetPlatformWebsiteById,
	"Platform Website"
);
cached_reference_lookup!(
	get_player_perspective_by_id_cached,
	PlayerPerspective,
	get_player_perspective_by_id,
	GetPlayerPerspectiveById,
	"Player Perspective"
);
cached_lookup!(
	get_popularity_primitive_by_id_cached,
	PopularityPrimitive,
	get_popularity_primitive_by_id,
	GetPopularityPrimitiveById,
	"Popularity Primitive"
);
cached_reference_lookup!(
	get_popularity_type_by_id_cached,
	PopularityType,
	get_popularity_type_by_id,
	GetPopularityTypeById,
	"Popularity Type"
);
cached_reference_lookup!(
	get_region_by_id_cached,
	Region,
	get_region_by_id,
	GetRegionById,
	"Region"
);
cached_lookup!(
	get_release_date_by_id_cached,
	ReleaseDate,
	get_release_date_by_id,
	GetReleaseDateById,
	"Release Date"
);
cached_lookup!(
	get_release_date_status_by_id_cached,
	ReleaseDateStatus,
	get_release_date_status_by_id,
	GetReleaseDateStatusById,
	"Release Date Status"
);
cached_lookup!(
	get_screenshot_by_id_cached,
	Screenshot,
	get_screenshot_by_id,
	GetScreenshotById,
	"Screenshot"
);
cached_reference_lookup!(
	get_theme_by_id_cached,
	Theme,
	get_theme_by_id,
	GetThemeById,
	"Theme"
);
cached_lookup!(
	get_website_by_id_cached,
	Website,
	get_website_by_id,
	GetWebsiteById,
	"Website"
);

#[derive(Debug, Clone, Copy)]
enum IgdbCacheType {
	GetAgeRatingById,
	GetAgeRatingCategoryById,
	GetAgeRatingContentDescriptionTypeById,
	GetAgeRatingContentDescriptionV2ById,
	GetAgeRatingOrganizationById,
	GetAlternativeNameById,
	GetArtworkById,
	GetArtworkTypeById,
	GetCharacterMugShotById,
	GetCollectionById,
	GetCompanySizeById,
	GetCompanyStatusById,
	GetCompanyTypeById,
	GetCompanyTypeHistoryById,
	GetCoverById,
	GetDateFormatById,
	GetEntityTypeById,
	GetExternalGameById,
	GetExternalGameSourceById,
	GetFranchiseById,
	GetGameById,
	GetGameBySlug,
	SearchGameByName,
	GetGameReleaseFormatById,
	GetGameStatusById,
	GetGameTimeToBeatById,
	GetGameTypeById,
	GetGenreById,
	GetPlatformTypeById,
	GetReleaseDateRegionById,
	GetReportById,
	GetReportTypeById,
	GetWebsiteTypeById,
	GetCharacterById,
	GetCharacterGenderById,
	GetCharacterSpeciesById,
	GetCollectionMembershipById,
	GetCollectionMembershipTypeById,
	GetCollectionRelationById,
	GetCollectionRelationTypeById,
	GetCollectionTypeById,
	GetCompanyById,
	GetCompanyLogoById,
	GetCompanyWebsiteById,
	GetEventById,
	GetEventLogoById,
	GetEventNetworkById,
	GetGameEngineById,
	GetGameEngineLogoById,
	GetGameLocalizationById,
	GetGameModeById,
	GetGameVersionById,
	GetGameVersionFeatureById,
	GetGameVersionFeatureValueById,
	GetGameVideoById,
	GetInvolvedCompanyById,
	GetKeywordById,
	GetLanguageById,
	GetLanguageSupportById,
	GetLanguageSupportTypeById,
	GetMultiplayerModeById,
	GetNetworkTypeById,
	GetPlatformById,
	GetPlatformFamilyById,
	GetPlatformLogoById,
	GetPlatformVersionById,
	GetPlatformVersionCompanyById,
	GetPlatformVersionReleaseDateById,
	GetPlatformWebsiteById,
	GetPlayerPerspectiveById,
	GetPopularityPrimitiveById,
	GetPopularityTypeById,
	GetRegionById,
	GetReleaseDateById,
	GetReleaseDateStatusById,
	GetScreenshotById,
	GetThemeById,
	GetWebsiteById,
}

impl IgdbCacheType {
	fn segment(&self) -> &'static str {
		match self {
			Self::GetAgeRatingById => "age_rating",
			Self::GetAgeRatingCategoryById => "age_rating_category",
			Self::GetAgeRatingContentDescriptionTypeById => "age_rating_content_description_type",
			Self::GetAgeRatingContentDescriptionV2ById => "age_rating_content_description_v2",
			Self::GetAgeRatingOrganizationById => "age_rating_organization",
			Self::GetAlternativeNameById => "alternative_name",
			Self::GetArtworkById => "artwork",
			Self::GetArtworkTypeById => "artwork_type",
			Self::GetCharacterMugShotById => "character_mug_shot",
			Self::GetCollectionById => "collection",
			Self::GetCompanySizeById => "company_size",
			Self::GetCompanyStatusById => "company_status",
			Self::GetCompanyTypeById => "company_type",
			Self::GetCompanyTypeHistoryById => "company_type_history",
			Self::GetCoverById => "cover",
			Self::GetDateFormatById => "date_format",
			Self::GetEntityTypeById => "entity_type",
			Self::GetExternalGameById => "external_game",
			Self::GetExternalGameSourceById => "external_game_source",
			Self::GetFranchiseById => "franchise",
			Self::GetGameById => "game",
			Self::GetGameBySlug => "game:slug",
			Self::SearchGameByName => "game:search",
			Self::GetGameReleaseFormatById => "game_release_format",
			Self::GetGameStatusById => "game_status",
			Self::GetGameTimeToBeatById => "game_time_to_beat",
			Self::GetGameTypeById => "game_type",
			Self::GetGenreById => "genre",
			Self::GetPlatformTypeById => "platform_type",
			Self::GetReleaseDateRegionById => "release_date_region",
			Self::GetReportById => "report",
			Self::GetReportTypeById => "report_type",
			Self::GetWebsiteTypeById => "website_type",
			Self::GetCharacterById => "character",
			Self::GetCharacterGenderById => "character_gender",
			Self::GetCharacterSpeciesById => "character_species",
			Self::GetCollectionMembershipById => "collection_membership",
			Self::GetCollectionMembershipTypeById => "collection_membership_type",
			Self::GetCollectionRelationById => "collection_relation",
			Self::GetCollectionRelationTypeById => "collection_relation_type",
			Self::GetCollectionTypeById => "collection_type",
			Self::GetCompanyById => "company",
			Self::GetCompanyLogoById => "company_logo",
			Self::GetCompanyWebsiteById => "company_website",
			Self::GetEventById => "event",
			Self::GetEventLogoById => "event_logo",
			Self::GetEventNetworkById => "event_network",
			Self::GetGameEngineById => "game_engine",
			Self::GetGameEngineLogoById => "game_engine_logo",
			Self::GetGameLocalizationById => "game_localization",
			Self::GetGameModeById => "game_mode",
			Self::GetGameVersionById => "game_version",
			Self::GetGameVersionFeatureById => "game_version_feature",
			Self::GetGameVersionFeatureValueById => "game_version_feature_value",
			Self::GetGameVideoById => "game_video",
			Self::GetInvolvedCompanyById => "involved_company",
			Self::GetKeywordById => "keyword",
			Self::GetLanguageById => "language",
			Self::GetLanguageSupportById => "language_support",
			Self::GetLanguageSupportTypeById => "language_support_type",
			Self::GetMultiplayerModeById => "multiplayer_mode",
			Self::GetNetworkTypeById => "network_type",
			Self::GetPlatformById => "platform",
			Self::GetPlatformFamilyById => "platform_family",
			Self::GetPlatformLogoById => "platform_logo",
			Self::GetPlatformVersionById => "platform_version",
			Self::GetPlatformVersionCompanyById => "platform_version_company",
			Self::GetPlatformVersionReleaseDateById => "platform_version_release_date",
			Self::GetPlatformWebsiteById => "platform_website",
			Self::GetPlayerPerspectiveById => "player_perspective",
			Self::GetPopularityPrimitiveById => "popularity_primitive",
			Self::GetPopularityTypeById => "popularity_type",
			Self::GetRegionById => "region",
			Self::GetReleaseDateById => "release_date",
			Self::GetReleaseDateStatusById => "release_date_status",
			Self::GetScreenshotById => "screenshot",
			Self::GetThemeById => "theme",
			Self::GetWebsiteById => "website",
		}
	}
}

impl CacheKey for IgdbCacheType {
	fn get_cache_key(&self, identifier: &str) -> String {
		format!(
			"{CACHE_PREFIX}:cache:{CACHE_KEY_VERSION}:igdb:{}:{identifier}",
			self.segment()
		)
	}
}
