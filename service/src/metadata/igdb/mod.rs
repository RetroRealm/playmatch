use crate::http::abstraction::USER_AGENT;
use crate::metadata::igdb::constants::{API_URL, IGDB_ROUTE_GAMES};
use crate::metadata::igdb::model::Game;
use chrono::{DateTime, Utc};
use log::debug;
use oauth2::basic::{BasicClient, BasicTokenResponse};
use oauth2::reqwest::async_http_client;
use oauth2::AuthType::RequestBody;
use oauth2::{AuthUrl, ClientId, ClientSecret, TokenResponse, TokenUrl};
use reqwest::header::HeaderMap;
use reqwest::{Client, Method, Url};
use serde::de::DeserializeOwned;
use std::time::Duration;
use tower::limit::RateLimit;
use tower::{Service, ServiceExt};

mod constants;
pub mod model;

pub struct IgdbClient {
	client: Client,
	oauth2: BasicClient,
	service: RateLimit<Client>,
	client_id: String,
	token_response: Option<BasicTokenResponse>,
	last_token_request: Option<DateTime<Utc>>,
}

impl IgdbClient {
	pub fn new(client_id: String, client_secret: String, client: Client) -> anyhow::Result<Self> {
		let service = tower::ServiceBuilder::new()
			.rate_limit(4, Duration::from_secs(1))
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
			oauth2: oauth2_client,
			service,
			client_id,
			token_response: None,
			last_token_request: None,
		})
	}

	pub async fn get_game_by_id(&mut self, id: i64) -> anyhow::Result<Option<Game>> {
		self.get_single_by_id(IGDB_ROUTE_GAMES, id).await
	}

	pub async fn get_games_by_id(&mut self, ids: Vec<i64>) -> anyhow::Result<Vec<Game>> {
		self.get_vec_by_ids(IGDB_ROUTE_GAMES, ids).await
	}

	pub async fn search_game_by_name(&mut self, name: String) -> anyhow::Result<Vec<Game>> {
		self.do_request_parsed::<Vec<Game>>(
			Method::POST,
			IGDB_ROUTE_GAMES,
			None,
			Some(&format!("search \"{}\";", name)),
			Some(""),
		)
		.await
	}

	async fn get_single_by_id<T: DeserializeOwned>(
		&mut self,
		endpoint: &str,
		id: i64,
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
		&mut self,
		endpoint: &str,
		ids: Vec<i64>,
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

	async fn refresh_token(&mut self) -> anyhow::Result<()> {
		let token_result = self
			.oauth2
			.exchange_client_credentials()
			.request_async(async_http_client)
			.await?;

		debug!("Token result: {:?}", token_result);

		self.last_token_request = Some(Utc::now());
		self.token_response = Some(token_result);

		Ok(())
	}

	async fn refresh_token_if_needed(&mut self) -> anyhow::Result<()> {
		if self.token_response.is_none() {
			self.refresh_token().await?;
			return Ok(());
		}

		if let Some(token) = &self.token_response {
			if let Some(last_request) = self.last_token_request {
				let now = Utc::now();
				let diff = now - last_request;

				if diff.num_seconds() + 60 > token.expires_in().unwrap_or_default().as_secs() as i64
				{
					self.refresh_token().await?;
				}
			}
		}

		Ok(())
	}

	async fn do_request_parsed<T: DeserializeOwned>(
		&mut self,
		method: Method,
		path: &str,
		fields_clause: Option<&str>,
		where_clause: Option<&str>,
		limit_clause: Option<&str>,
	) -> anyhow::Result<T> {
		// TODO: Change this method to not refresh and instead refresh in an own async task so that we do not need a mutable reference to self and can save on that mutex lock

		self.refresh_token_if_needed().await?;

		let mut headers = HeaderMap::new();
		headers.insert("Client-Id", self.client_id.parse()?);
		unsafe {
			headers.insert("User-Agent", USER_AGENT.parse()?);
		}
		headers.insert(
			"Authorization",
			format!(
				"Bearer {}",
				self.token_response
					.as_ref()
					.unwrap()
					.access_token()
					.secret()
					.as_str()
			)
			.parse()?,
		);

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

		let res = self.service.ready().await?.call(req).await?;

		Ok(res.json().await?)
	}
}
