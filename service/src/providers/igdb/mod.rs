use crate::config::http::REQWEST_DEFAULT_USER_AGENT;
use crate::http::abstraction::RetryPolicy;
use crate::providers::igdb::constants::{
	API_URL, IGDB_MAX_RETRIES, IGDB_RATELIMIT_AMOUNT, IGDB_RATELIMIT_DURATION_MS,
	IGDB_ROUTE_AGE_RATING_CATEGORIES, IGDB_ROUTE_AGE_RATING_CONTENT_DESCRIPTION_TYPES,
	IGDB_ROUTE_AGE_RATING_CONTENT_DESCRIPTIONS_V2, IGDB_ROUTE_AGE_RATING_ORGANIZATIONS,
	IGDB_ROUTE_AGE_RATINGS, IGDB_ROUTE_ALTERNATIVE_NAMES, IGDB_ROUTE_ARTWORK_TYPES,
	IGDB_ROUTE_ARTWORKS, IGDB_ROUTE_CHARACTER_GENDERS, IGDB_ROUTE_CHARACTER_MUG_SHOTS,
	IGDB_ROUTE_CHARACTER_SPECIES, IGDB_ROUTE_CHARACTERS, IGDB_ROUTE_COLLECTION_MEMBERSHIP_TYPES,
	IGDB_ROUTE_COLLECTION_MEMBERSHIPS, IGDB_ROUTE_COLLECTION_RELATION_TYPES,
	IGDB_ROUTE_COLLECTION_RELATIONS, IGDB_ROUTE_COLLECTION_TYPES, IGDB_ROUTE_COLLECTIONS,
	IGDB_ROUTE_COMPANIES, IGDB_ROUTE_COMPANY_LOGOS, IGDB_ROUTE_COMPANY_SIZES,
	IGDB_ROUTE_COMPANY_STATUSES, IGDB_ROUTE_COMPANY_TYPE_HISTORIES, IGDB_ROUTE_COMPANY_TYPES,
	IGDB_ROUTE_COMPANY_WEBSITES, IGDB_ROUTE_COVERS, IGDB_ROUTE_DATE_FORMATS,
	IGDB_ROUTE_ENTITY_TYPES, IGDB_ROUTE_EVENT_LOGOS, IGDB_ROUTE_EVENT_NETWORKS, IGDB_ROUTE_EVENTS,
	IGDB_ROUTE_EXTERNAL_GAME_SOURCES, IGDB_ROUTE_EXTERNAL_GAMES, IGDB_ROUTE_FRANCHISES,
	IGDB_ROUTE_GAME_ENGINE_LOGOS, IGDB_ROUTE_GAME_ENGINES, IGDB_ROUTE_GAME_LOCALIZATIONS,
	IGDB_ROUTE_GAME_MODES, IGDB_ROUTE_GAME_RELEASE_FORMATS, IGDB_ROUTE_GAME_STATUSES,
	IGDB_ROUTE_GAME_TIME_TO_BEATS, IGDB_ROUTE_GAME_TYPES, IGDB_ROUTE_GAME_VERSION_FEATURE_VALUES,
	IGDB_ROUTE_GAME_VERSION_FEATURES, IGDB_ROUTE_GAME_VERSIONS, IGDB_ROUTE_GAME_VIDEOS,
	IGDB_ROUTE_GAMES, IGDB_ROUTE_GENRES, IGDB_ROUTE_INVOLVED_COMPANIES, IGDB_ROUTE_KEYWORDS,
	IGDB_ROUTE_LANGUAGE_SUPPORT_TYPES, IGDB_ROUTE_LANGUAGE_SUPPORTS, IGDB_ROUTE_LANGUAGES,
	IGDB_ROUTE_MULTIPLAYER_MODES, IGDB_ROUTE_NETWORK_TYPES, IGDB_ROUTE_PLATFORM_FAMILIES,
	IGDB_ROUTE_PLATFORM_LOGOS, IGDB_ROUTE_PLATFORM_TYPES, IGDB_ROUTE_PLATFORM_VERSION_COMPANIES,
	IGDB_ROUTE_PLATFORM_VERSION_RELEASE_DATES, IGDB_ROUTE_PLATFORM_VERSIONS,
	IGDB_ROUTE_PLATFORM_WEBSITES, IGDB_ROUTE_PLATFORMS, IGDB_ROUTE_PLAYER_PERSPECTIVES,
	IGDB_ROUTE_POPULARITY_PRIMITIVES, IGDB_ROUTE_POPULARITY_TYPES, IGDB_ROUTE_REGIONS,
	IGDB_ROUTE_RELEASE_DATE_REGIONS, IGDB_ROUTE_RELEASE_DATE_STATUSES, IGDB_ROUTE_RELEASE_DATES,
	IGDB_ROUTE_REPORT_TYPES, IGDB_ROUTE_REPORTS, IGDB_ROUTE_SCREENSHOTS, IGDB_ROUTE_THEMES,
	IGDB_ROUTE_WEBSITE_TYPES, IGDB_ROUTE_WEBSITES,
};
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
use chrono::{DateTime, Utc};
use log::debug;
use oauth2::AuthType::RequestBody;
use oauth2::basic::{
	BasicClient, BasicErrorResponse, BasicRevocationErrorResponse, BasicTokenIntrospectionResponse,
	BasicTokenResponse,
};
use oauth2::{
	AuthUrl, ClientId, ClientSecret, EndpointNotSet, EndpointSet, StandardRevocableToken,
	TokenResponse, TokenUrl,
};
use reqwest::header::HeaderMap;
use reqwest::{Client, Method, Url};
use serde::de::DeserializeOwned;
use std::ops::DerefMut;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tower::limit::{RateLimit, RateLimitLayer};
use tower::retry::Retry;
use tower::{Service, ServiceBuilder, ServiceExt};

mod apicalypse;
pub mod cache;
mod constants;
pub mod matching;
pub mod model;

struct OAuth2Handler {
	oauth2: oauth2::Client<
		BasicErrorResponse,
		BasicTokenResponse,
		BasicTokenIntrospectionResponse,
		StandardRevocableToken,
		BasicRevocationErrorResponse,
		EndpointSet,
		EndpointNotSet,
		EndpointNotSet,
		EndpointNotSet,
		EndpointSet,
	>,
	token_response: Option<BasicTokenResponse>,
	last_token_request: Option<DateTime<Utc>>,
}

pub struct IgdbClient {
	client: Client,
	service: Mutex<RateLimit<Retry<RetryPolicy, Client>>>,
	oauth2handler: RwLock<OAuth2Handler>,
	client_id: String,
}

impl IgdbClient {
	pub fn new(client_id: String, client_secret: String, client: Client) -> anyhow::Result<Self> {
		let rate_limit_layer = RateLimitLayer::new(
			IGDB_RATELIMIT_AMOUNT,
			Duration::from_millis(IGDB_RATELIMIT_DURATION_MS),
		);
		let retry_layer = tower::retry::RetryLayer::new(RetryPolicy(IGDB_MAX_RETRIES));

		let service = ServiceBuilder::new()
			.layer(rate_limit_layer)
			.layer(retry_layer)
			.service(client.clone());

		let mut oauth2_client = BasicClient::new(ClientId::new(client_id.clone()))
			.set_client_secret(ClientSecret::new(client_secret.clone()))
			.set_auth_uri(AuthUrl::new(
				"https://id.twitch.tv/oauth2/token".to_string(),
			)?)
			.set_token_uri(TokenUrl::new(
				"https://id.twitch.tv/oauth2/token".to_string(),
			)?);

		oauth2_client = oauth2_client.set_auth_type(RequestBody);

		Ok(Self {
			client,
			client_id,
			service: Mutex::new(service),
			oauth2handler: RwLock::new(OAuth2Handler {
				oauth2: oauth2_client,
				token_response: None,
				last_token_request: None,
			}),
		})
	}

	pub async fn search_company_by_name(&self, name: &str) -> anyhow::Result<Vec<Company>> {
		let literal = apicalypse::quote(name)?;
		self.do_request_parsed::<Vec<Company>>(
			Method::POST,
			IGDB_ROUTE_COMPANIES,
			None,
			Some(&format!("where name = {literal};")),
			Some(""),
		)
		.await
	}

	pub async fn search_platforms_by_name(&self, name: &str) -> anyhow::Result<Vec<Platform>> {
		let literal = apicalypse::quote(name)?;
		self.do_request_parsed::<Vec<Platform>>(
			Method::POST,
			IGDB_ROUTE_PLATFORMS,
			None,
			Some(&format!("search {literal};")),
			Some(""),
		)
		.await
	}

	pub async fn get_game_by_id(&self, id: i32) -> anyhow::Result<Option<Game>> {
		self.get_single_by_id(IGDB_ROUTE_GAMES, id).await
	}

	pub async fn get_game_by_slug(&self, slug: &str) -> anyhow::Result<Option<Game>> {
		let literal = apicalypse::quote(slug)?;
		let mut res = self
			.do_request_parsed::<Vec<Game>>(
				Method::POST,
				IGDB_ROUTE_GAMES,
				None,
				Some(&format!("where slug = {literal};")),
				Some("limit 1;"),
			)
			.await?;

		Ok(res.pop())
	}

	pub async fn get_games_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<Game>> {
		self.get_vec_by_ids(IGDB_ROUTE_GAMES, ids).await
	}

	pub async fn search_game_by_name(&self, name: &str) -> anyhow::Result<Vec<Game>> {
		let literal = apicalypse::quote(name)?;
		self.do_request_parsed::<Vec<Game>>(
			Method::POST,
			IGDB_ROUTE_GAMES,
			None,
			Some(&format!("search {literal};")),
			Some(""),
		)
		.await
	}

	pub async fn search_game_by_name_and_platform(
		&self,
		name: &str,
		platform_id: i32,
	) -> anyhow::Result<Vec<Game>> {
		let literal = apicalypse::quote(name)?;
		self.do_request_parsed::<Vec<Game>>(
			Method::POST,
			IGDB_ROUTE_GAMES,
			None,
			Some(&format!(
				"where platforms = ({platform_id}); search {literal};"
			)),
			Some(""),
		)
		.await
	}

	pub async fn get_age_rating_by_id(&self, id: i32) -> anyhow::Result<Option<AgeRating>> {
		self.get_single_by_id(IGDB_ROUTE_AGE_RATINGS, id).await
	}

	pub async fn get_age_ratings_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<AgeRating>> {
		self.get_vec_by_ids(IGDB_ROUTE_AGE_RATINGS, ids).await
	}

	pub async fn get_alternative_name_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<AlternativeName>> {
		self.get_single_by_id(IGDB_ROUTE_ALTERNATIVE_NAMES, id)
			.await
	}

	pub async fn get_alternative_names_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<AlternativeName>> {
		self.get_vec_by_ids(IGDB_ROUTE_ALTERNATIVE_NAMES, ids).await
	}

	pub async fn get_artwork_by_id(&self, id: i32) -> anyhow::Result<Option<Artwork>> {
		self.get_single_by_id(IGDB_ROUTE_ARTWORKS, id).await
	}

	pub async fn get_artworks_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<Artwork>> {
		self.get_vec_by_ids(IGDB_ROUTE_ARTWORKS, ids).await
	}

	pub async fn get_collection_by_id(&self, id: i32) -> anyhow::Result<Option<Collection>> {
		self.get_single_by_id(IGDB_ROUTE_COLLECTIONS, id).await
	}

	pub async fn get_collections_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<Collection>> {
		self.get_vec_by_ids(IGDB_ROUTE_COLLECTIONS, ids).await
	}

	pub async fn get_cover_by_id(&self, id: i32) -> anyhow::Result<Option<Cover>> {
		self.get_single_by_id(IGDB_ROUTE_COVERS, id).await
	}

	pub async fn get_covers_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<Cover>> {
		self.get_vec_by_ids(IGDB_ROUTE_COVERS, ids).await
	}

	pub async fn get_external_game_by_id(&self, id: i32) -> anyhow::Result<Option<ExternalGame>> {
		self.get_single_by_id(IGDB_ROUTE_EXTERNAL_GAMES, id).await
	}

	pub async fn get_external_games_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<ExternalGame>> {
		self.get_vec_by_ids(IGDB_ROUTE_EXTERNAL_GAMES, ids).await
	}

	pub async fn get_franchise_by_id(&self, id: i32) -> anyhow::Result<Option<Franchise>> {
		self.get_single_by_id(IGDB_ROUTE_FRANCHISES, id).await
	}

	pub async fn get_franchises_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<Franchise>> {
		self.get_vec_by_ids(IGDB_ROUTE_FRANCHISES, ids).await
	}

	pub async fn get_genre_by_id(&self, id: i32) -> anyhow::Result<Option<Genre>> {
		self.get_single_by_id(IGDB_ROUTE_GENRES, id).await
	}

	pub async fn get_genres_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<Genre>> {
		self.get_vec_by_ids(IGDB_ROUTE_GENRES, ids).await
	}

	pub async fn get_age_rating_category_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<AgeRatingCategory>> {
		self.get_single_by_id(IGDB_ROUTE_AGE_RATING_CATEGORIES, id)
			.await
	}

	pub async fn get_age_rating_categories_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<AgeRatingCategory>> {
		self.get_vec_by_ids(IGDB_ROUTE_AGE_RATING_CATEGORIES, ids)
			.await
	}

	pub async fn get_age_rating_content_description_v2_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<AgeRatingContentDescriptionV2>> {
		self.get_single_by_id(IGDB_ROUTE_AGE_RATING_CONTENT_DESCRIPTIONS_V2, id)
			.await
	}

	pub async fn get_age_rating_content_descriptions_v2_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<AgeRatingContentDescriptionV2>> {
		self.get_vec_by_ids(IGDB_ROUTE_AGE_RATING_CONTENT_DESCRIPTIONS_V2, ids)
			.await
	}

	pub async fn get_age_rating_content_description_type_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<AgeRatingContentDescriptionType>> {
		self.get_single_by_id(IGDB_ROUTE_AGE_RATING_CONTENT_DESCRIPTION_TYPES, id)
			.await
	}

	pub async fn get_age_rating_content_description_types_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<AgeRatingContentDescriptionType>> {
		self.get_vec_by_ids(IGDB_ROUTE_AGE_RATING_CONTENT_DESCRIPTION_TYPES, ids)
			.await
	}

	pub async fn get_age_rating_organization_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<AgeRatingOrganization>> {
		self.get_single_by_id(IGDB_ROUTE_AGE_RATING_ORGANIZATIONS, id)
			.await
	}

	pub async fn get_age_rating_organizations_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<AgeRatingOrganization>> {
		self.get_vec_by_ids(IGDB_ROUTE_AGE_RATING_ORGANIZATIONS, ids)
			.await
	}

	pub async fn get_company_status_by_id(&self, id: i32) -> anyhow::Result<Option<CompanyStatus>> {
		self.get_single_by_id(IGDB_ROUTE_COMPANY_STATUSES, id).await
	}

	pub async fn get_company_statuses_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<CompanyStatus>> {
		self.get_vec_by_ids(IGDB_ROUTE_COMPANY_STATUSES, ids).await
	}

	pub async fn get_date_format_by_id(&self, id: i32) -> anyhow::Result<Option<DateFormat>> {
		self.get_single_by_id(IGDB_ROUTE_DATE_FORMATS, id).await
	}

	pub async fn get_date_formats_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<DateFormat>> {
		self.get_vec_by_ids(IGDB_ROUTE_DATE_FORMATS, ids).await
	}

	pub async fn get_external_game_source_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<ExternalGameSource>> {
		self.get_single_by_id(IGDB_ROUTE_EXTERNAL_GAME_SOURCES, id)
			.await
	}

	pub async fn get_external_game_sources_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<ExternalGameSource>> {
		self.get_vec_by_ids(IGDB_ROUTE_EXTERNAL_GAME_SOURCES, ids)
			.await
	}

	pub async fn get_game_release_format_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<GameReleaseFormat>> {
		self.get_single_by_id(IGDB_ROUTE_GAME_RELEASE_FORMATS, id)
			.await
	}

	pub async fn get_game_release_formats_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<GameReleaseFormat>> {
		self.get_vec_by_ids(IGDB_ROUTE_GAME_RELEASE_FORMATS, ids)
			.await
	}

	pub async fn get_game_status_by_id(&self, id: i32) -> anyhow::Result<Option<GameStatus>> {
		self.get_single_by_id(IGDB_ROUTE_GAME_STATUSES, id).await
	}

	pub async fn get_game_statuses_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<GameStatus>> {
		self.get_vec_by_ids(IGDB_ROUTE_GAME_STATUSES, ids).await
	}

	pub async fn get_game_type_by_id(&self, id: i32) -> anyhow::Result<Option<GameType>> {
		self.get_single_by_id(IGDB_ROUTE_GAME_TYPES, id).await
	}

	pub async fn get_game_types_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<GameType>> {
		self.get_vec_by_ids(IGDB_ROUTE_GAME_TYPES, ids).await
	}

	pub async fn get_platform_type_by_id(&self, id: i32) -> anyhow::Result<Option<PlatformType>> {
		self.get_single_by_id(IGDB_ROUTE_PLATFORM_TYPES, id).await
	}

	pub async fn get_platform_types_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<PlatformType>> {
		self.get_vec_by_ids(IGDB_ROUTE_PLATFORM_TYPES, ids).await
	}

	pub async fn get_release_date_region_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<ReleaseDateRegion>> {
		self.get_single_by_id(IGDB_ROUTE_RELEASE_DATE_REGIONS, id)
			.await
	}

	pub async fn get_release_date_regions_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<ReleaseDateRegion>> {
		self.get_vec_by_ids(IGDB_ROUTE_RELEASE_DATE_REGIONS, ids)
			.await
	}

	pub async fn get_website_type_by_id(&self, id: i32) -> anyhow::Result<Option<WebsiteType>> {
		self.get_single_by_id(IGDB_ROUTE_WEBSITE_TYPES, id).await
	}

	pub async fn get_website_types_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<WebsiteType>> {
		self.get_vec_by_ids(IGDB_ROUTE_WEBSITE_TYPES, ids).await
	}

	pub async fn get_artwork_type_by_id(&self, id: i32) -> anyhow::Result<Option<ArtworkType>> {
		self.get_single_by_id(IGDB_ROUTE_ARTWORK_TYPES, id).await
	}

	pub async fn get_artwork_types_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<ArtworkType>> {
		self.get_vec_by_ids(IGDB_ROUTE_ARTWORK_TYPES, ids).await
	}

	pub async fn get_character_mug_shot_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<CharacterMugShot>> {
		self.get_single_by_id(IGDB_ROUTE_CHARACTER_MUG_SHOTS, id)
			.await
	}

	pub async fn get_character_mug_shots_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<CharacterMugShot>> {
		self.get_vec_by_ids(IGDB_ROUTE_CHARACTER_MUG_SHOTS, ids)
			.await
	}

	pub async fn get_company_size_by_id(&self, id: i32) -> anyhow::Result<Option<CompanySize>> {
		self.get_single_by_id(IGDB_ROUTE_COMPANY_SIZES, id).await
	}

	pub async fn get_company_sizes_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<CompanySize>> {
		self.get_vec_by_ids(IGDB_ROUTE_COMPANY_SIZES, ids).await
	}

	pub async fn get_company_type_by_id(&self, id: i32) -> anyhow::Result<Option<CompanyType>> {
		self.get_single_by_id(IGDB_ROUTE_COMPANY_TYPES, id).await
	}

	pub async fn get_company_types_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<CompanyType>> {
		self.get_vec_by_ids(IGDB_ROUTE_COMPANY_TYPES, ids).await
	}

	pub async fn get_company_type_history_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<CompanyTypeHistory>> {
		self.get_single_by_id(IGDB_ROUTE_COMPANY_TYPE_HISTORIES, id)
			.await
	}

	pub async fn get_company_type_histories_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<CompanyTypeHistory>> {
		self.get_vec_by_ids(IGDB_ROUTE_COMPANY_TYPE_HISTORIES, ids)
			.await
	}

	pub async fn get_entity_type_by_id(&self, id: i32) -> anyhow::Result<Option<EntityType>> {
		self.get_single_by_id(IGDB_ROUTE_ENTITY_TYPES, id).await
	}

	pub async fn get_entity_types_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<EntityType>> {
		self.get_vec_by_ids(IGDB_ROUTE_ENTITY_TYPES, ids).await
	}

	pub async fn get_game_time_to_beat_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<GameTimeToBeat>> {
		self.get_single_by_id(IGDB_ROUTE_GAME_TIME_TO_BEATS, id)
			.await
	}

	pub async fn get_game_time_to_beats_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<GameTimeToBeat>> {
		self.get_vec_by_ids(IGDB_ROUTE_GAME_TIME_TO_BEATS, ids)
			.await
	}

	pub async fn get_report_by_id(&self, id: i32) -> anyhow::Result<Option<Report>> {
		self.get_single_by_id(IGDB_ROUTE_REPORTS, id).await
	}

	pub async fn get_reports_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<Report>> {
		self.get_vec_by_ids(IGDB_ROUTE_REPORTS, ids).await
	}

	pub async fn get_report_type_by_id(&self, id: i32) -> anyhow::Result<Option<ReportType>> {
		self.get_single_by_id(IGDB_ROUTE_REPORT_TYPES, id).await
	}

	pub async fn get_report_types_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<ReportType>> {
		self.get_vec_by_ids(IGDB_ROUTE_REPORT_TYPES, ids).await
	}

	pub async fn get_character_by_id(&self, id: i32) -> anyhow::Result<Option<Character>> {
		self.get_single_by_id(IGDB_ROUTE_CHARACTERS, id).await
	}

	pub async fn get_characters_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<Character>> {
		self.get_vec_by_ids(IGDB_ROUTE_CHARACTERS, ids).await
	}

	pub async fn get_character_gender_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<CharacterGender>> {
		self.get_single_by_id(IGDB_ROUTE_CHARACTER_GENDERS, id)
			.await
	}

	pub async fn get_character_genders_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<CharacterGender>> {
		self.get_vec_by_ids(IGDB_ROUTE_CHARACTER_GENDERS, ids).await
	}

	pub async fn get_character_species_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<CharacterSpecies>> {
		self.get_single_by_id(IGDB_ROUTE_CHARACTER_SPECIES, id)
			.await
	}

	pub async fn get_character_species_by_ids(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<CharacterSpecies>> {
		self.get_vec_by_ids(IGDB_ROUTE_CHARACTER_SPECIES, ids).await
	}

	pub async fn get_collection_membership_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<CollectionMembership>> {
		self.get_single_by_id(IGDB_ROUTE_COLLECTION_MEMBERSHIPS, id)
			.await
	}

	pub async fn get_collection_memberships_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<CollectionMembership>> {
		self.get_vec_by_ids(IGDB_ROUTE_COLLECTION_MEMBERSHIPS, ids)
			.await
	}

	pub async fn get_collection_membership_type_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<CollectionMembershipType>> {
		self.get_single_by_id(IGDB_ROUTE_COLLECTION_MEMBERSHIP_TYPES, id)
			.await
	}

	pub async fn get_collection_membership_types_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<CollectionMembershipType>> {
		self.get_vec_by_ids(IGDB_ROUTE_COLLECTION_MEMBERSHIP_TYPES, ids)
			.await
	}

	pub async fn get_collection_relation_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<CollectionRelation>> {
		self.get_single_by_id(IGDB_ROUTE_COLLECTION_RELATIONS, id)
			.await
	}

	pub async fn get_collection_relations_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<CollectionRelation>> {
		self.get_vec_by_ids(IGDB_ROUTE_COLLECTION_RELATIONS, ids)
			.await
	}

	pub async fn get_collection_relation_type_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<CollectionRelationType>> {
		self.get_single_by_id(IGDB_ROUTE_COLLECTION_RELATION_TYPES, id)
			.await
	}

	pub async fn get_collection_relation_types_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<CollectionRelationType>> {
		self.get_vec_by_ids(IGDB_ROUTE_COLLECTION_RELATION_TYPES, ids)
			.await
	}

	pub async fn get_collection_type_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<CollectionType>> {
		self.get_single_by_id(IGDB_ROUTE_COLLECTION_TYPES, id).await
	}

	pub async fn get_collection_types_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<CollectionType>> {
		self.get_vec_by_ids(IGDB_ROUTE_COLLECTION_TYPES, ids).await
	}

	pub async fn get_company_by_id(&self, id: i32) -> anyhow::Result<Option<Company>> {
		self.get_single_by_id(IGDB_ROUTE_COMPANIES, id).await
	}

	pub async fn get_companies_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<Company>> {
		self.get_vec_by_ids(IGDB_ROUTE_COMPANIES, ids).await
	}

	pub async fn get_company_logo_by_id(&self, id: i32) -> anyhow::Result<Option<CompanyLogo>> {
		self.get_single_by_id(IGDB_ROUTE_COMPANY_LOGOS, id).await
	}

	pub async fn get_company_logos_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<CompanyLogo>> {
		self.get_vec_by_ids(IGDB_ROUTE_COMPANY_LOGOS, ids).await
	}

	pub async fn get_company_website_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<CompanyWebsite>> {
		self.get_single_by_id(IGDB_ROUTE_COMPANY_WEBSITES, id).await
	}

	pub async fn get_company_websites_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<CompanyWebsite>> {
		self.get_vec_by_ids(IGDB_ROUTE_COMPANY_WEBSITES, ids).await
	}

	pub async fn get_event_by_id(&self, id: i32) -> anyhow::Result<Option<Event>> {
		self.get_single_by_id(IGDB_ROUTE_EVENTS, id).await
	}

	pub async fn get_events_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<Event>> {
		self.get_vec_by_ids(IGDB_ROUTE_EVENTS, ids).await
	}

	pub async fn get_event_logo_by_id(&self, id: i32) -> anyhow::Result<Option<EventLogo>> {
		self.get_single_by_id(IGDB_ROUTE_EVENT_LOGOS, id).await
	}

	pub async fn get_event_logos_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<EventLogo>> {
		self.get_vec_by_ids(IGDB_ROUTE_EVENT_LOGOS, ids).await
	}

	pub async fn get_event_network_by_id(&self, id: i32) -> anyhow::Result<Option<EventNetwork>> {
		self.get_single_by_id(IGDB_ROUTE_EVENT_NETWORKS, id).await
	}

	pub async fn get_event_networks_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<EventNetwork>> {
		self.get_vec_by_ids(IGDB_ROUTE_EVENT_NETWORKS, ids).await
	}

	pub async fn get_game_engine_by_id(&self, id: i32) -> anyhow::Result<Option<GameEngine>> {
		self.get_single_by_id(IGDB_ROUTE_GAME_ENGINES, id).await
	}

	pub async fn get_game_engines_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<GameEngine>> {
		self.get_vec_by_ids(IGDB_ROUTE_GAME_ENGINES, ids).await
	}

	pub async fn get_game_engine_logo_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<GameEngineLogo>> {
		self.get_single_by_id(IGDB_ROUTE_GAME_ENGINE_LOGOS, id)
			.await
	}

	pub async fn get_game_engine_logos_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<GameEngineLogo>> {
		self.get_vec_by_ids(IGDB_ROUTE_GAME_ENGINE_LOGOS, ids).await
	}

	pub async fn get_game_localization_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<GameLocalization>> {
		self.get_single_by_id(IGDB_ROUTE_GAME_LOCALIZATIONS, id)
			.await
	}

	pub async fn get_game_localizations_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<GameLocalization>> {
		self.get_vec_by_ids(IGDB_ROUTE_GAME_LOCALIZATIONS, ids)
			.await
	}

	pub async fn get_game_mode_by_id(&self, id: i32) -> anyhow::Result<Option<GameMode>> {
		self.get_single_by_id(IGDB_ROUTE_GAME_MODES, id).await
	}

	pub async fn get_game_modes_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<GameMode>> {
		self.get_vec_by_ids(IGDB_ROUTE_GAME_MODES, ids).await
	}

	pub async fn get_game_version_by_id(&self, id: i32) -> anyhow::Result<Option<GameVersion>> {
		self.get_single_by_id(IGDB_ROUTE_GAME_VERSIONS, id).await
	}

	pub async fn get_game_versions_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<GameVersion>> {
		self.get_vec_by_ids(IGDB_ROUTE_GAME_VERSIONS, ids).await
	}

	pub async fn get_game_version_feature_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<GameVersionFeature>> {
		self.get_single_by_id(IGDB_ROUTE_GAME_VERSION_FEATURES, id)
			.await
	}

	pub async fn get_game_version_features_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<GameVersionFeature>> {
		self.get_vec_by_ids(IGDB_ROUTE_GAME_VERSION_FEATURES, ids)
			.await
	}

	pub async fn get_game_version_feature_value_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<GameVersionFeatureValue>> {
		self.get_single_by_id(IGDB_ROUTE_GAME_VERSION_FEATURE_VALUES, id)
			.await
	}

	pub async fn get_game_version_feature_values_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<GameVersionFeatureValue>> {
		self.get_vec_by_ids(IGDB_ROUTE_GAME_VERSION_FEATURE_VALUES, ids)
			.await
	}

	pub async fn get_game_video_by_id(&self, id: i32) -> anyhow::Result<Option<GameVideo>> {
		self.get_single_by_id(IGDB_ROUTE_GAME_VIDEOS, id).await
	}

	pub async fn get_game_videos_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<GameVideo>> {
		self.get_vec_by_ids(IGDB_ROUTE_GAME_VIDEOS, ids).await
	}

	pub async fn get_involved_company_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<InvolvedCompany>> {
		self.get_single_by_id(IGDB_ROUTE_INVOLVED_COMPANIES, id)
			.await
	}

	pub async fn get_involved_companies_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<InvolvedCompany>> {
		self.get_vec_by_ids(IGDB_ROUTE_INVOLVED_COMPANIES, ids)
			.await
	}

	pub async fn get_keyword_by_id(&self, id: i32) -> anyhow::Result<Option<Keyword>> {
		self.get_single_by_id(IGDB_ROUTE_KEYWORDS, id).await
	}

	pub async fn get_keywords_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<Keyword>> {
		self.get_vec_by_ids(IGDB_ROUTE_KEYWORDS, ids).await
	}

	pub async fn get_language_by_id(&self, id: i32) -> anyhow::Result<Option<Language>> {
		self.get_single_by_id(IGDB_ROUTE_LANGUAGES, id).await
	}

	pub async fn get_languages_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<Language>> {
		self.get_vec_by_ids(IGDB_ROUTE_LANGUAGES, ids).await
	}

	pub async fn get_language_support_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<LanguageSupport>> {
		self.get_single_by_id(IGDB_ROUTE_LANGUAGE_SUPPORTS, id)
			.await
	}

	pub async fn get_language_supports_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<LanguageSupport>> {
		self.get_vec_by_ids(IGDB_ROUTE_LANGUAGE_SUPPORTS, ids).await
	}

	pub async fn get_language_support_type_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<LanguageSupportType>> {
		self.get_single_by_id(IGDB_ROUTE_LANGUAGE_SUPPORT_TYPES, id)
			.await
	}

	pub async fn get_language_support_types_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<LanguageSupportType>> {
		self.get_vec_by_ids(IGDB_ROUTE_LANGUAGE_SUPPORT_TYPES, ids)
			.await
	}

	pub async fn get_multiplayer_mode_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<MultiplayerMode>> {
		self.get_single_by_id(IGDB_ROUTE_MULTIPLAYER_MODES, id)
			.await
	}

	pub async fn get_multiplayer_modes_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<MultiplayerMode>> {
		self.get_vec_by_ids(IGDB_ROUTE_MULTIPLAYER_MODES, ids).await
	}

	pub async fn get_network_type_by_id(&self, id: i32) -> anyhow::Result<Option<NetworkType>> {
		self.get_single_by_id(IGDB_ROUTE_NETWORK_TYPES, id).await
	}

	pub async fn get_network_types_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<NetworkType>> {
		self.get_vec_by_ids(IGDB_ROUTE_NETWORK_TYPES, ids).await
	}

	pub async fn get_platform_by_id(&self, id: i32) -> anyhow::Result<Option<Platform>> {
		self.get_single_by_id(IGDB_ROUTE_PLATFORMS, id).await
	}

	pub async fn get_platforms_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<Platform>> {
		self.get_vec_by_ids(IGDB_ROUTE_PLATFORMS, ids).await
	}

	pub async fn get_platform_family_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<PlatformFamily>> {
		self.get_single_by_id(IGDB_ROUTE_PLATFORM_FAMILIES, id)
			.await
	}

	pub async fn get_platform_families_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<PlatformFamily>> {
		self.get_vec_by_ids(IGDB_ROUTE_PLATFORM_FAMILIES, ids).await
	}

	pub async fn get_platform_logo_by_id(&self, id: i32) -> anyhow::Result<Option<PlatformLogo>> {
		self.get_single_by_id(IGDB_ROUTE_PLATFORM_LOGOS, id).await
	}

	pub async fn get_platform_logos_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<PlatformLogo>> {
		self.get_vec_by_ids(IGDB_ROUTE_PLATFORM_LOGOS, ids).await
	}

	pub async fn get_platform_version_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<PlatformVersion>> {
		self.get_single_by_id(IGDB_ROUTE_PLATFORM_VERSIONS, id)
			.await
	}

	pub async fn get_platform_versions_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<PlatformVersion>> {
		self.get_vec_by_ids(IGDB_ROUTE_PLATFORM_VERSIONS, ids).await
	}

	pub async fn get_platform_version_company_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<PlatformVersionCompany>> {
		self.get_single_by_id(IGDB_ROUTE_PLATFORM_VERSION_COMPANIES, id)
			.await
	}

	pub async fn get_platform_version_companies_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<PlatformVersionCompany>> {
		self.get_vec_by_ids(IGDB_ROUTE_PLATFORM_VERSION_COMPANIES, ids)
			.await
	}

	pub async fn get_platform_version_release_date_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<PlatformVersionReleaseDate>> {
		self.get_single_by_id(IGDB_ROUTE_PLATFORM_VERSION_RELEASE_DATES, id)
			.await
	}

	pub async fn get_platform_version_release_dates_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<PlatformVersionReleaseDate>> {
		self.get_vec_by_ids(IGDB_ROUTE_PLATFORM_VERSION_RELEASE_DATES, ids)
			.await
	}

	pub async fn get_platform_website_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<PlatformWebsite>> {
		self.get_single_by_id(IGDB_ROUTE_PLATFORM_WEBSITES, id)
			.await
	}

	pub async fn get_platform_websites_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<PlatformWebsite>> {
		self.get_vec_by_ids(IGDB_ROUTE_PLATFORM_WEBSITES, ids).await
	}

	pub async fn get_player_perspective_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<PlayerPerspective>> {
		self.get_single_by_id(IGDB_ROUTE_PLAYER_PERSPECTIVES, id)
			.await
	}

	pub async fn get_player_perspectives_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<PlayerPerspective>> {
		self.get_vec_by_ids(IGDB_ROUTE_PLAYER_PERSPECTIVES, ids)
			.await
	}

	pub async fn get_popularity_primitive_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<PopularityPrimitive>> {
		self.get_single_by_id(IGDB_ROUTE_POPULARITY_PRIMITIVES, id)
			.await
	}

	pub async fn get_popularity_primitives_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<PopularityPrimitive>> {
		self.get_vec_by_ids(IGDB_ROUTE_POPULARITY_PRIMITIVES, ids)
			.await
	}

	pub async fn get_popularity_type_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<PopularityType>> {
		self.get_single_by_id(IGDB_ROUTE_POPULARITY_TYPES, id).await
	}

	pub async fn get_popularity_types_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<PopularityType>> {
		self.get_vec_by_ids(IGDB_ROUTE_POPULARITY_TYPES, ids).await
	}

	pub async fn get_region_by_id(&self, id: i32) -> anyhow::Result<Option<Region>> {
		self.get_single_by_id(IGDB_ROUTE_REGIONS, id).await
	}

	pub async fn get_regions_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<Region>> {
		self.get_vec_by_ids(IGDB_ROUTE_REGIONS, ids).await
	}

	pub async fn get_release_date_by_id(&self, id: i32) -> anyhow::Result<Option<ReleaseDate>> {
		self.get_single_by_id(IGDB_ROUTE_RELEASE_DATES, id).await
	}

	pub async fn get_release_dates_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<ReleaseDate>> {
		self.get_vec_by_ids(IGDB_ROUTE_RELEASE_DATES, ids).await
	}

	pub async fn get_release_date_status_by_id(
		&self,
		id: i32,
	) -> anyhow::Result<Option<ReleaseDateStatus>> {
		self.get_single_by_id(IGDB_ROUTE_RELEASE_DATE_STATUSES, id)
			.await
	}

	pub async fn get_release_date_statuses_by_id(
		&self,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<ReleaseDateStatus>> {
		self.get_vec_by_ids(IGDB_ROUTE_RELEASE_DATE_STATUSES, ids)
			.await
	}

	pub async fn get_screenshot_by_id(&self, id: i32) -> anyhow::Result<Option<Screenshot>> {
		self.get_single_by_id(IGDB_ROUTE_SCREENSHOTS, id).await
	}

	pub async fn get_screenshots_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<Screenshot>> {
		self.get_vec_by_ids(IGDB_ROUTE_SCREENSHOTS, ids).await
	}

	pub async fn get_theme_by_id(&self, id: i32) -> anyhow::Result<Option<Theme>> {
		self.get_single_by_id(IGDB_ROUTE_THEMES, id).await
	}

	pub async fn get_themes_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<Theme>> {
		self.get_vec_by_ids(IGDB_ROUTE_THEMES, ids).await
	}

	pub async fn get_website_by_id(&self, id: i32) -> anyhow::Result<Option<Website>> {
		self.get_single_by_id(IGDB_ROUTE_WEBSITES, id).await
	}

	pub async fn get_websites_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<Website>> {
		self.get_vec_by_ids(IGDB_ROUTE_WEBSITES, ids).await
	}

	async fn get_single_by_id<T: DeserializeOwned>(
		&self,
		endpoint: &str,
		id: i32,
	) -> anyhow::Result<Option<T>> {
		let mut res = self
			.do_request_parsed::<Vec<T>>(
				Method::POST,
				endpoint,
				None,
				Some(&format!("where id = {id};")),
				Some("limit 1;"),
			)
			.await?;

		Ok(res.pop())
	}

	async fn get_vec_by_ids<T: DeserializeOwned>(
		&self,
		endpoint: &str,
		ids: Vec<i32>,
	) -> anyhow::Result<Vec<T>> {
		self.do_request_parsed::<Vec<T>>(
			Method::POST,
			endpoint,
			None,
			Some(&format!(
				"where id =({});",
				ids.iter()
					.map(|id| id.to_string())
					.collect::<Vec<String>>()
					.join(",")
			)),
			Some(""),
		)
		.await
	}

	async fn refresh_token(&self) -> anyhow::Result<()> {
		let mut handler = self.oauth2handler.write().await;
		let handler_ref = handler.deref_mut();

		let token_result = handler_ref
			.oauth2
			.exchange_client_credentials()
			.request_async(&self.client)
			.await?;

		debug!(
			"igdb oauth refresh ok: token_type={:?} expires_in={:?}",
			token_result.token_type(),
			token_result.expires_in(),
		);

		handler_ref.last_token_request = Some(Utc::now());
		handler_ref.token_response = Some(token_result);

		Ok(())
	}

	async fn refresh_token_if_needed(&self) -> anyhow::Result<()> {
		let oauth2 = self.oauth2handler.read().await;
		let oauth2_token = oauth2.token_response.as_ref();
		let oauth2_last_token_request = oauth2.last_token_request.as_ref();

		if oauth2_token.is_none() {
			drop(oauth2);
			return self.refresh_token_instrumented("initial").await;
		}

		if let Some(token) = oauth2_token
			&& let Some(last_request) = oauth2_last_token_request
		{
			let now = Utc::now();
			let diff = now - last_request;

			if diff.num_seconds() + 60 > token.expires_in().unwrap_or_default().as_secs() as i64 {
				drop(oauth2);
				return self.refresh_token_instrumented("expired").await;
			}
		}

		Ok(())
	}

	async fn refresh_token_instrumented(&self, trigger: &'static str) -> anyhow::Result<()> {
		match self.refresh_token().await {
			Ok(()) => {
				crate::metrics::record_metadata_token_refresh("igdb",trigger, "success");
				Ok(())
			}
			Err(e) => {
				crate::metrics::record_metadata_token_refresh("igdb",trigger, "error");
				Err(e)
			}
		}
	}

	async fn do_request_parsed<T: DeserializeOwned>(
		&self,
		method: Method,
		path: &str,
		fields_clause: Option<&str>,
		where_clause: Option<&str>,
		limit_clause: Option<&str>,
	) -> anyhow::Result<T> {
		let started = std::time::Instant::now();
		let result = self
			.do_request_parsed_inner::<T>(method, path, fields_clause, where_clause, limit_clause)
			.await;
		let outcome = if result.is_ok() { "success" } else { "error" };
		crate::metrics::record_metadata_request("igdb", path, outcome, started.elapsed().as_secs_f64());
		result
	}

	async fn do_request_parsed_inner<T: DeserializeOwned>(
		&self,
		method: Method,
		path: &str,
		fields_clause: Option<&str>,
		where_clause: Option<&str>,
		limit_clause: Option<&str>,
	) -> anyhow::Result<T> {
		self.refresh_token_if_needed().await?;

		let mut headers = HeaderMap::new();
		headers.insert("Client-Id", self.client_id.parse()?);
		headers.insert("User-Agent", REQWEST_DEFAULT_USER_AGENT.parse()?);
		let oauth2 = self.oauth2handler.read().await;
		let access_token = oauth2
			.token_response
			.as_ref()
			.unwrap()
			.access_token()
			.secret();

		headers.insert("Authorization", format!("Bearer {access_token}").parse()?);
		drop(oauth2);

		let req = self
			.client
			.request(method, Url::parse(format!("{API_URL}/{path}").as_str())?)
			.headers(headers)
			.body(format!(
				"{}{}{}",
				fields_clause.unwrap_or("fields *;"),
				where_clause.unwrap_or(""),
				limit_clause.unwrap_or("limit 1;")
			))
			.build()?;

		debug!("igdb request: {} {}", req.method(), req.url().path());
		if let Some(body) = req.body()
			&& let Some(bytes) = body.as_bytes()
		{
			debug!("igdb request body: {:?}", std::str::from_utf8(bytes)?);
		}

		let rate_limited_future = self.service.lock().await.ready().await?.call(req);
		// MutexGuard has to have been dropped here, so it's 2 statements
		let res = rate_limited_future.await?;

		let body = res.text().await?;
		if log::log_enabled!(log::Level::Debug) {
			let preview: String = body.chars().take(256).collect();
			debug!("igdb response (first 256 chars): {preview}");
		}

		Ok(serde_json::from_str(&body)?)
	}
}
