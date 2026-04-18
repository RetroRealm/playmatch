use crate::config::http::REQWEST_DEFAULT_USER_AGENT;
use crate::http::abstraction::RetryPolicy;
use crate::providers::igdb::constants::{
	API_URL, IGDB_MAX_RETRIES, IGDB_RATELIMIT_AMOUNT, IGDB_RATELIMIT_DURATION_MS,
	IGDB_ROUTE_AGE_RATING_CATEGORIES, IGDB_ROUTE_AGE_RATING_CONTENT_DESCRIPTION_TYPES,
	IGDB_ROUTE_AGE_RATING_CONTENT_DESCRIPTIONS_V2, IGDB_ROUTE_AGE_RATING_ORGANIZATIONS,
	IGDB_ROUTE_AGE_RATINGS, IGDB_ROUTE_ALTERNATIVE_NAMES, IGDB_ROUTE_ARTWORK_TYPES,
	IGDB_ROUTE_ARTWORKS, IGDB_ROUTE_CHARACTER_MUG_SHOTS, IGDB_ROUTE_COLLECTIONS,
	IGDB_ROUTE_COMPANIES, IGDB_ROUTE_COMPANY_SIZES, IGDB_ROUTE_COMPANY_STATUSES,
	IGDB_ROUTE_COMPANY_TYPE_HISTORIES, IGDB_ROUTE_COMPANY_TYPES, IGDB_ROUTE_COVERS,
	IGDB_ROUTE_DATE_FORMATS, IGDB_ROUTE_ENTITY_TYPES, IGDB_ROUTE_EXTERNAL_GAME_SOURCES,
	IGDB_ROUTE_EXTERNAL_GAMES, IGDB_ROUTE_FRANCHISES, IGDB_ROUTE_GAME_RELEASE_FORMATS,
	IGDB_ROUTE_GAME_STATUSES, IGDB_ROUTE_GAME_TIME_TO_BEATS, IGDB_ROUTE_GAME_TYPES,
	IGDB_ROUTE_GAMES, IGDB_ROUTE_GENRES, IGDB_ROUTE_PLATFORM_TYPES, IGDB_ROUTE_PLATFORMS,
	IGDB_ROUTE_RELEASE_DATE_REGIONS, IGDB_ROUTE_REPORT_TYPES, IGDB_ROUTE_REPORTS,
	IGDB_ROUTE_WEBSITE_TYPES,
};
use crate::providers::igdb::model::{
	AgeRating, AgeRatingCategory, AgeRatingContentDescriptionType, AgeRatingContentDescriptionV2,
	AgeRatingOrganization, AlternativeName, Artwork, ArtworkType, CharacterMugShot, Collection,
	Company, CompanySize, CompanyStatus, CompanyType, CompanyTypeHistory, Cover, DateFormat,
	EntityType, ExternalGame, ExternalGameSource, Franchise, Game, GameReleaseFormat, GameStatus,
	GameTimeToBeat, GameType, Genre, Platform, PlatformType, ReleaseDateRegion, Report, ReportType,
	WebsiteType,
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
		self.do_request_parsed::<Vec<Company>>(
			Method::POST,
			IGDB_ROUTE_COMPANIES,
			None,
			Some(&format!("where name = \"{name}\";")),
			Some(""),
		)
		.await
	}

	pub async fn search_platforms_by_name(&self, name: &str) -> anyhow::Result<Vec<Platform>> {
		self.do_request_parsed::<Vec<Platform>>(
			Method::POST,
			IGDB_ROUTE_PLATFORMS,
			None,
			Some(&format!("search \"{name}\";")),
			Some(""),
		)
		.await
	}

	pub async fn get_game_by_id(&self, id: i32) -> anyhow::Result<Option<Game>> {
		self.get_single_by_id(IGDB_ROUTE_GAMES, id).await
	}

	pub async fn get_game_by_slug(&self, slug: &str) -> anyhow::Result<Option<Game>> {
		let mut res = self
			.do_request_parsed::<Vec<Game>>(
				Method::POST,
				IGDB_ROUTE_GAMES,
				None,
				Some(&format!("where slug = \"{slug}\";")),
				Some("limit 1;"),
			)
			.await?;

		Ok(res.pop())
	}

	pub async fn get_games_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<Game>> {
		self.get_vec_by_ids(IGDB_ROUTE_GAMES, ids).await
	}

	pub async fn search_game_by_name(&self, name: &str) -> anyhow::Result<Vec<Game>> {
		self.do_request_parsed::<Vec<Game>>(
			Method::POST,
			IGDB_ROUTE_GAMES,
			None,
			Some(&format!("search \"{name}\";")),
			Some(""),
		)
		.await
	}

	pub async fn search_game_by_name_and_platform(
		&self,
		name: &str,
		platform_id: i32,
	) -> anyhow::Result<Vec<Game>> {
		self.do_request_parsed::<Vec<Game>>(
			Method::POST,
			IGDB_ROUTE_GAMES,
			None,
			Some(&format!(
				"where platforms = ({platform_id}); search \"{name}\";"
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

		debug!("Token result: {token_result:?}");

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
				crate::metrics::record_igdb_token_refresh(trigger, "success");
				Ok(())
			}
			Err(e) => {
				crate::metrics::record_igdb_token_refresh(trigger, "error");
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
		crate::metrics::record_igdb_request(path, outcome, started.elapsed().as_secs_f64());
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

		debug!("Request: {req:?}");
		if let Some(body) = req.body()
			&& let Some(bytes) = body.as_bytes()
		{
			debug!("Request body: {:?}", std::str::from_utf8(bytes)?);
		}

		let rate_limited_future = self.service.lock().await.ready().await?.call(req);
		// MutexGuard has to have been dropped here, so it's 2 statements
		let res = rate_limited_future.await?;

		let body = res.text().await?;
		debug!("Response: {body}");

		Ok(serde_json::from_str(&body)?)
	}
}
