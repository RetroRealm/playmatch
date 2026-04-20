use crate::cache::{
	CACHE_PREFIX, CacheKey, deserialize_option_redis_value, serialize_option_redis_value,
};
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
use log::debug;
use redis::AsyncTypedCommands;
use redis::aio::MultiplexedConnection;
use std::time::Duration;

const IGDB_CACHE_LIFETIME: u64 = Duration::from_secs(60 * 60 * 24).as_secs(); // 1 day

macro_rules! cached_lookup {
	($fn_name:ident, $ty:ty, $fetch:ident, $variant:ident, $label:literal) => {
		pub async fn $fn_name(
			igdb_client: &IgdbClient,
			redis_conn: &mut MultiplexedConnection,
			id: i32,
		) -> anyhow::Result<Option<$ty>> {
			let cache_key = IgdbCacheType::$variant.get_cache_key(&id.to_string());

			if let Ok(Some(cached_val)) = redis_conn.get(&cache_key).await {
				debug!("igdb Cache hit for {} with id: {}", $label, id);
				$crate::metrics::record_cache_hit("igdb", $label);
				redis_conn
					.expire(&cache_key, IGDB_CACHE_LIFETIME as i64)
					.await?;
				let deserialized = deserialize_option_redis_value(cached_val)?;
				return Ok(deserialized);
			}
			debug!("igdb Cache miss for {} with id: {}", $label, id);
			$crate::metrics::record_cache_miss("igdb", $label);

			let value = igdb_client.$fetch(id).await?;

			redis_conn
				.set_ex(
					&cache_key,
					serialize_option_redis_value(value.clone())?,
					IGDB_CACHE_LIFETIME,
				)
				.await?;

			Ok(value)
		}
	};
}

macro_rules! cached_lookup_by_slug {
	($fn_name:ident, $ty:ty, $fetch:ident, $variant:ident, $label:literal) => {
		pub async fn $fn_name(
			igdb_client: &IgdbClient,
			redis_conn: &mut MultiplexedConnection,
			slug: String,
		) -> anyhow::Result<Option<$ty>> {
			let cache_key = IgdbCacheType::$variant.get_cache_key(&slug);

			if let Ok(Some(cached_val)) = redis_conn.get(&cache_key).await {
				debug!("igdb Cache hit for {} with slug: {}", $label, slug);
				$crate::metrics::record_cache_hit("igdb", $label);
				redis_conn
					.expire(&cache_key, IGDB_CACHE_LIFETIME as i64)
					.await?;
				let deserialized = deserialize_option_redis_value(cached_val)?;
				return Ok(deserialized);
			}
			debug!("igdb Cache miss for {} with slug: {}", $label, slug);
			$crate::metrics::record_cache_miss("igdb", $label);

			let value = igdb_client.$fetch(&slug).await?;

			redis_conn
				.set_ex(
					&cache_key,
					serialize_option_redis_value(value.clone())?,
					IGDB_CACHE_LIFETIME,
				)
				.await?;

			Ok(value)
		}
	};
}

macro_rules! cached_search {
	($fn_name:ident, $ty:ty, $fetch:ident, $variant:ident, $label:literal) => {
		pub async fn $fn_name(
			igdb_client: &IgdbClient,
			redis_conn: &mut MultiplexedConnection,
			query: String,
		) -> anyhow::Result<Vec<$ty>> {
			let cache_key = IgdbCacheType::$variant.get_cache_key(&query);

			if let Ok(Some(cached_val)) = redis_conn.get(&cache_key).await {
				debug!("igdb Cache hit for {} query: {}", $label, query);
				$crate::metrics::record_cache_hit("igdb", $label);
				redis_conn
					.expire(&cache_key, IGDB_CACHE_LIFETIME as i64)
					.await?;
				let deserialized: Vec<$ty> = serde_json::from_str(&cached_val)?;
				return Ok(deserialized);
			}
			debug!("igdb Cache miss for {} query: {}", $label, query);
			$crate::metrics::record_cache_miss("igdb", $label);

			let values = igdb_client.$fetch(&query).await?;

			redis_conn
				.set_ex(
					&cache_key,
					serde_json::to_string(&values)?,
					IGDB_CACHE_LIFETIME,
				)
				.await?;

			Ok(values)
		}
	};
}

cached_lookup!(
	get_age_rating_by_id_cached,
	AgeRating,
	get_age_rating_by_id,
	GetAgeRatingById,
	"Age Rating"
);
cached_lookup!(
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
cached_lookup!(
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
cached_lookup!(
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
cached_lookup!(
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
cached_lookup!(
	get_game_type_by_id_cached,
	GameType,
	get_game_type_by_id,
	GetGameTypeById,
	"Game Type"
);
cached_lookup!(
	get_genre_by_id_cached,
	Genre,
	get_genre_by_id,
	GetGenreById,
	"Genre"
);
cached_lookup!(
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
cached_lookup!(
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
cached_lookup!(
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
cached_lookup!(
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
cached_lookup!(
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
cached_lookup!(
	get_popularity_type_by_id_cached,
	PopularityType,
	get_popularity_type_by_id,
	GetPopularityTypeById,
	"Popularity Type"
);
cached_lookup!(
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
cached_lookup!(
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

impl CacheKey for IgdbCacheType {
	fn get_cache_key(&self, identifier: &str) -> String {
		match self {
			IgdbCacheType::GetAgeRatingById => {
				format!("{CACHE_PREFIX}:cache:igdb:age_rating:{identifier}")
			}
			IgdbCacheType::GetAgeRatingCategoryById => {
				format!("{CACHE_PREFIX}:cache:igdb:age_rating_category:{identifier}")
			}
			IgdbCacheType::GetAgeRatingContentDescriptionTypeById => {
				format!(
					"{CACHE_PREFIX}:cache:igdb:age_rating_content_description_type:{identifier}"
				)
			}
			IgdbCacheType::GetAgeRatingContentDescriptionV2ById => {
				format!("{CACHE_PREFIX}:cache:igdb:age_rating_content_description_v2:{identifier}")
			}
			IgdbCacheType::GetAgeRatingOrganizationById => {
				format!("{CACHE_PREFIX}:cache:igdb:age_rating_organization:{identifier}")
			}
			IgdbCacheType::GetAlternativeNameById => {
				format!("{CACHE_PREFIX}:cache:igdb:alternative_name:{identifier}")
			}
			IgdbCacheType::GetArtworkById => {
				format!("{CACHE_PREFIX}:cache:igdb:artwork:{identifier}")
			}
			IgdbCacheType::GetArtworkTypeById => {
				format!("{CACHE_PREFIX}:cache:igdb:artwork_type:{identifier}")
			}
			IgdbCacheType::GetCharacterMugShotById => {
				format!("{CACHE_PREFIX}:cache:igdb:character_mug_shot:{identifier}")
			}
			IgdbCacheType::GetCollectionById => {
				format!("{CACHE_PREFIX}:cache:igdb:collection:{identifier}")
			}
			IgdbCacheType::GetCompanySizeById => {
				format!("{CACHE_PREFIX}:cache:igdb:company_size:{identifier}")
			}
			IgdbCacheType::GetCompanyStatusById => {
				format!("{CACHE_PREFIX}:cache:igdb:company_status:{identifier}")
			}
			IgdbCacheType::GetCompanyTypeById => {
				format!("{CACHE_PREFIX}:cache:igdb:company_type:{identifier}")
			}
			IgdbCacheType::GetCompanyTypeHistoryById => {
				format!("{CACHE_PREFIX}:cache:igdb:company_type_history:{identifier}")
			}
			IgdbCacheType::GetCoverById => {
				format!("{CACHE_PREFIX}:cache:igdb:cover:{identifier}")
			}
			IgdbCacheType::GetDateFormatById => {
				format!("{CACHE_PREFIX}:cache:igdb:date_format:{identifier}")
			}
			IgdbCacheType::GetEntityTypeById => {
				format!("{CACHE_PREFIX}:cache:igdb:entity_type:{identifier}")
			}
			IgdbCacheType::GetExternalGameById => {
				format!("{CACHE_PREFIX}:cache:igdb:external_game:{identifier}")
			}
			IgdbCacheType::GetExternalGameSourceById => {
				format!("{CACHE_PREFIX}:cache:igdb:external_game_source:{identifier}")
			}
			IgdbCacheType::GetFranchiseById => {
				format!("{CACHE_PREFIX}:cache:igdb:franchise:{identifier}")
			}
			IgdbCacheType::GetGameById => {
				format!("{CACHE_PREFIX}:cache:igdb:game:{identifier}")
			}
			IgdbCacheType::GetGameBySlug => {
				format!("{CACHE_PREFIX}:cache:igdb:game:slug:{identifier}")
			}
			IgdbCacheType::SearchGameByName => {
				format!("{CACHE_PREFIX}:cache:igdb:game:search:{identifier}")
			}
			IgdbCacheType::GetGameReleaseFormatById => {
				format!("{CACHE_PREFIX}:cache:igdb:game_release_format:{identifier}")
			}
			IgdbCacheType::GetGameStatusById => {
				format!("{CACHE_PREFIX}:cache:igdb:game_status:{identifier}")
			}
			IgdbCacheType::GetGameTimeToBeatById => {
				format!("{CACHE_PREFIX}:cache:igdb:game_time_to_beat:{identifier}")
			}
			IgdbCacheType::GetGameTypeById => {
				format!("{CACHE_PREFIX}:cache:igdb:game_type:{identifier}")
			}
			IgdbCacheType::GetGenreById => {
				format!("{CACHE_PREFIX}:cache:igdb:genre:{identifier}")
			}
			IgdbCacheType::GetPlatformTypeById => {
				format!("{CACHE_PREFIX}:cache:igdb:platform_type:{identifier}")
			}
			IgdbCacheType::GetReleaseDateRegionById => {
				format!("{CACHE_PREFIX}:cache:igdb:release_date_region:{identifier}")
			}
			IgdbCacheType::GetReportById => {
				format!("{CACHE_PREFIX}:cache:igdb:report:{identifier}")
			}
			IgdbCacheType::GetReportTypeById => {
				format!("{CACHE_PREFIX}:cache:igdb:report_type:{identifier}")
			}
			IgdbCacheType::GetWebsiteTypeById => {
				format!("{CACHE_PREFIX}:cache:igdb:website_type:{identifier}")
			}
			IgdbCacheType::GetCharacterById => {
				format!("{CACHE_PREFIX}:cache:igdb:character:{identifier}")
			}
			IgdbCacheType::GetCharacterGenderById => {
				format!("{CACHE_PREFIX}:cache:igdb:character_gender:{identifier}")
			}
			IgdbCacheType::GetCharacterSpeciesById => {
				format!("{CACHE_PREFIX}:cache:igdb:character_species:{identifier}")
			}
			IgdbCacheType::GetCollectionMembershipById => {
				format!("{CACHE_PREFIX}:cache:igdb:collection_membership:{identifier}")
			}
			IgdbCacheType::GetCollectionMembershipTypeById => {
				format!("{CACHE_PREFIX}:cache:igdb:collection_membership_type:{identifier}")
			}
			IgdbCacheType::GetCollectionRelationById => {
				format!("{CACHE_PREFIX}:cache:igdb:collection_relation:{identifier}")
			}
			IgdbCacheType::GetCollectionRelationTypeById => {
				format!("{CACHE_PREFIX}:cache:igdb:collection_relation_type:{identifier}")
			}
			IgdbCacheType::GetCollectionTypeById => {
				format!("{CACHE_PREFIX}:cache:igdb:collection_type:{identifier}")
			}
			IgdbCacheType::GetCompanyById => {
				format!("{CACHE_PREFIX}:cache:igdb:company:{identifier}")
			}
			IgdbCacheType::GetCompanyLogoById => {
				format!("{CACHE_PREFIX}:cache:igdb:company_logo:{identifier}")
			}
			IgdbCacheType::GetCompanyWebsiteById => {
				format!("{CACHE_PREFIX}:cache:igdb:company_website:{identifier}")
			}
			IgdbCacheType::GetEventById => {
				format!("{CACHE_PREFIX}:cache:igdb:event:{identifier}")
			}
			IgdbCacheType::GetEventLogoById => {
				format!("{CACHE_PREFIX}:cache:igdb:event_logo:{identifier}")
			}
			IgdbCacheType::GetEventNetworkById => {
				format!("{CACHE_PREFIX}:cache:igdb:event_network:{identifier}")
			}
			IgdbCacheType::GetGameEngineById => {
				format!("{CACHE_PREFIX}:cache:igdb:game_engine:{identifier}")
			}
			IgdbCacheType::GetGameEngineLogoById => {
				format!("{CACHE_PREFIX}:cache:igdb:game_engine_logo:{identifier}")
			}
			IgdbCacheType::GetGameLocalizationById => {
				format!("{CACHE_PREFIX}:cache:igdb:game_localization:{identifier}")
			}
			IgdbCacheType::GetGameModeById => {
				format!("{CACHE_PREFIX}:cache:igdb:game_mode:{identifier}")
			}
			IgdbCacheType::GetGameVersionById => {
				format!("{CACHE_PREFIX}:cache:igdb:game_version:{identifier}")
			}
			IgdbCacheType::GetGameVersionFeatureById => {
				format!("{CACHE_PREFIX}:cache:igdb:game_version_feature:{identifier}")
			}
			IgdbCacheType::GetGameVersionFeatureValueById => {
				format!("{CACHE_PREFIX}:cache:igdb:game_version_feature_value:{identifier}")
			}
			IgdbCacheType::GetGameVideoById => {
				format!("{CACHE_PREFIX}:cache:igdb:game_video:{identifier}")
			}
			IgdbCacheType::GetInvolvedCompanyById => {
				format!("{CACHE_PREFIX}:cache:igdb:involved_company:{identifier}")
			}
			IgdbCacheType::GetKeywordById => {
				format!("{CACHE_PREFIX}:cache:igdb:keyword:{identifier}")
			}
			IgdbCacheType::GetLanguageById => {
				format!("{CACHE_PREFIX}:cache:igdb:language:{identifier}")
			}
			IgdbCacheType::GetLanguageSupportById => {
				format!("{CACHE_PREFIX}:cache:igdb:language_support:{identifier}")
			}
			IgdbCacheType::GetLanguageSupportTypeById => {
				format!("{CACHE_PREFIX}:cache:igdb:language_support_type:{identifier}")
			}
			IgdbCacheType::GetMultiplayerModeById => {
				format!("{CACHE_PREFIX}:cache:igdb:multiplayer_mode:{identifier}")
			}
			IgdbCacheType::GetNetworkTypeById => {
				format!("{CACHE_PREFIX}:cache:igdb:network_type:{identifier}")
			}
			IgdbCacheType::GetPlatformById => {
				format!("{CACHE_PREFIX}:cache:igdb:platform:{identifier}")
			}
			IgdbCacheType::GetPlatformFamilyById => {
				format!("{CACHE_PREFIX}:cache:igdb:platform_family:{identifier}")
			}
			IgdbCacheType::GetPlatformLogoById => {
				format!("{CACHE_PREFIX}:cache:igdb:platform_logo:{identifier}")
			}
			IgdbCacheType::GetPlatformVersionById => {
				format!("{CACHE_PREFIX}:cache:igdb:platform_version:{identifier}")
			}
			IgdbCacheType::GetPlatformVersionCompanyById => {
				format!("{CACHE_PREFIX}:cache:igdb:platform_version_company:{identifier}")
			}
			IgdbCacheType::GetPlatformVersionReleaseDateById => {
				format!("{CACHE_PREFIX}:cache:igdb:platform_version_release_date:{identifier}")
			}
			IgdbCacheType::GetPlatformWebsiteById => {
				format!("{CACHE_PREFIX}:cache:igdb:platform_website:{identifier}")
			}
			IgdbCacheType::GetPlayerPerspectiveById => {
				format!("{CACHE_PREFIX}:cache:igdb:player_perspective:{identifier}")
			}
			IgdbCacheType::GetPopularityPrimitiveById => {
				format!("{CACHE_PREFIX}:cache:igdb:popularity_primitive:{identifier}")
			}
			IgdbCacheType::GetPopularityTypeById => {
				format!("{CACHE_PREFIX}:cache:igdb:popularity_type:{identifier}")
			}
			IgdbCacheType::GetRegionById => {
				format!("{CACHE_PREFIX}:cache:igdb:region:{identifier}")
			}
			IgdbCacheType::GetReleaseDateById => {
				format!("{CACHE_PREFIX}:cache:igdb:release_date:{identifier}")
			}
			IgdbCacheType::GetReleaseDateStatusById => {
				format!("{CACHE_PREFIX}:cache:igdb:release_date_status:{identifier}")
			}
			IgdbCacheType::GetScreenshotById => {
				format!("{CACHE_PREFIX}:cache:igdb:screenshot:{identifier}")
			}
			IgdbCacheType::GetThemeById => {
				format!("{CACHE_PREFIX}:cache:igdb:theme:{identifier}")
			}
			IgdbCacheType::GetWebsiteById => {
				format!("{CACHE_PREFIX}:cache:igdb:website:{identifier}")
			}
		}
	}
}
