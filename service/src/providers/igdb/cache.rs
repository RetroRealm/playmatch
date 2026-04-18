use crate::cache::{
	CACHE_PREFIX, CacheKey, deserialize_option_redis_value, serialize_option_redis_value,
};
use crate::providers::igdb::IgdbClient;
use crate::providers::igdb::model::{
	AgeRating, AgeRatingCategory, AgeRatingContentDescriptionType, AgeRatingContentDescriptionV2,
	AgeRatingOrganization, AlternativeName, Artwork, ArtworkType, CharacterMugShot, Collection,
	CompanySize, CompanyStatus, CompanyType, CompanyTypeHistory, Cover, DateFormat, EntityType,
	ExternalGame, ExternalGameSource, Franchise, Game, GameReleaseFormat, GameStatus,
	GameTimeToBeat, GameType, Genre, PlatformType, ReleaseDateRegion, Report, ReportType,
	WebsiteType,
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
				redis_conn
					.expire(&cache_key, IGDB_CACHE_LIFETIME as i64)
					.await?;
				let deserialized = deserialize_option_redis_value(cached_val)?;
				return Ok(deserialized);
			}
			debug!("igdb Cache miss for {} with id: {}", $label, id);

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
				redis_conn
					.expire(&cache_key, IGDB_CACHE_LIFETIME as i64)
					.await?;
				let deserialized = deserialize_option_redis_value(cached_val)?;
				return Ok(deserialized);
			}
			debug!("igdb Cache miss for {} with slug: {}", $label, slug);

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
				redis_conn
					.expire(&cache_key, IGDB_CACHE_LIFETIME as i64)
					.await?;
				let deserialized: Vec<$ty> = serde_json::from_str(&cached_val)?;
				return Ok(deserialized);
			}
			debug!("igdb Cache miss for {} query: {}", $label, query);

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
		}
	}
}
