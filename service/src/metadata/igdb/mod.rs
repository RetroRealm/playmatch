use crate::http::abstraction::{RetryPolicy, USER_AGENT};
use crate::metadata::igdb::constants::{
	API_URL, IGDB_MAX_RETRIES, IGDB_RATELIMIT_AMOUNT, IGDB_RATELIMIT_DURATION_MS,
	IGDB_ROUTE_AGE_RATINGS, IGDB_ROUTE_ALTERNATIVE_NAMES, IGDB_ROUTE_ARTWORKS,
	IGDB_ROUTE_COLLECTIONS, IGDB_ROUTE_COMPANIES, IGDB_ROUTE_COVERS, IGDB_ROUTE_EXTERNAL_GAMES,
	IGDB_ROUTE_FRANCHISES, IGDB_ROUTE_GAMES, IGDB_ROUTE_GENRES, IGDB_ROUTE_PLATFORMS,
};
use crate::metadata::igdb::model::{
	AgeRating, AlternativeName, Artwork, Collection, Company, Cover, ExternalGame, Franchise, Game,
	Genre, Platform,
};
use chrono::{DateTime, Utc};
use log::debug;
use oauth2::basic::{BasicClient, BasicTokenResponse};
use oauth2::reqwest::async_http_client;
use oauth2::AuthType::RequestBody;
use oauth2::{AuthUrl, ClientId, ClientSecret, TokenResponse, TokenUrl};
use reqwest::header::HeaderMap;
use reqwest::{Client, Method, Url};
use serde::de::DeserializeOwned;
use std::ops::DerefMut;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tower::limit::{RateLimit, RateLimitLayer};
use tower::retry::Retry;
use tower::{Service, ServiceBuilder, ServiceExt};

mod constants;
pub mod model;

struct OAuth2Handler {
	oauth2: BasicClient,
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

		let mut oauth2_client = BasicClient::new(
			ClientId::new(client_id.clone()),
			Some(ClientSecret::new(client_secret.clone())),
			AuthUrl::new("https://id.twitch.tv/oauth2/token".to_string())?,
			Some(TokenUrl::new(
				"https://id.twitch.tv/oauth2/token".to_string(),
			)?),
		);

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
			Some(&format!("where name =  \"{}\";", name)),
			Some(""),
		)
		.await
	}

	pub async fn search_platforms_by_name(&self, name: &str) -> anyhow::Result<Vec<Platform>> {
		self.do_request_parsed::<Vec<Platform>>(
			Method::POST,
			IGDB_ROUTE_PLATFORMS,
			None,
			Some(&format!("search \"{}\";", name)),
			Some(""),
		)
		.await
	}

	pub async fn get_game_by_id(&self, id: i32) -> anyhow::Result<Option<Game>> {
		self.get_single_by_id(IGDB_ROUTE_GAMES, id).await
	}

	pub async fn get_games_by_id(&self, ids: Vec<i32>) -> anyhow::Result<Vec<Game>> {
		self.get_vec_by_ids(IGDB_ROUTE_GAMES, ids).await
	}

	pub async fn search_game_by_name(&self, name: &str) -> anyhow::Result<Vec<Game>> {
		self.do_request_parsed::<Vec<Game>>(
			Method::POST,
			IGDB_ROUTE_GAMES,
			None,
			Some(&format!("search \"{}\";", name)),
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
				"where platforms = ({}); search \"{}\";",
				platform_id, name
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
				Some(&format!("where id = {};", id)),
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
			.request_async(async_http_client)
			.await?;

		debug!("Token result: {:?}", token_result);

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
			self.refresh_token().await?;
			return Ok(());
		}

		if let Some(token) = oauth2_token {
			if let Some(last_request) = oauth2_last_token_request {
				let now = Utc::now();
				let diff = now - last_request;

				if diff.num_seconds() + 60 > token.expires_in().unwrap_or_default().as_secs() as i64
				{
					drop(oauth2);
					self.refresh_token().await?;
				}
			}
		}

		Ok(())
	}

	async fn do_request_parsed<T: DeserializeOwned>(
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
		// Safety: The User Agent is only mutated on startup and is a constant string
		unsafe {
			headers.insert("User-Agent", USER_AGENT.parse()?);
		}
		let oauth2 = self.oauth2handler.read().await;
		let access_token = oauth2
			.token_response
			.as_ref()
			.unwrap()
			.access_token()
			.secret();

		headers.insert("Authorization", format!("Bearer {}", access_token).parse()?);
		drop(oauth2);

		let req = self
			.client
			.request(
				method,
				Url::parse(format!("{}/{}", API_URL, path).as_str())?,
			)
			.headers(headers)
			.body(format!(
				"{}{}{}",
				fields_clause.unwrap_or("fields *;"),
				where_clause.unwrap_or(""),
				limit_clause.unwrap_or("limit 1;")
			))
			.build()?;

		debug!("Request: {:?}", req);
		if let Some(body) = req.body() {
			if let Some(bytes) = body.as_bytes() {
				debug!("Request body: {:?}", std::str::from_utf8(bytes)?);
			}
		}

		let rate_limited_future = self.service.lock().await.ready().await?.call(req);
		// MutexGuard has to have been dropped here, so it's 2 statements
		let res = rate_limited_future.await?;

		let body = res.text().await?;
		debug!("Response: {}", body);

		Ok(serde_json::from_str(&body)?)
	}
}
