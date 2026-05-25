use crate::config::http::REQWEST_DEFAULT_USER_AGENT;
use crate::http::abstraction::RetryPolicy;
use crate::providers::mobygames::model::{
	MgCoversResp, MgGame, MgGamesResp, MgGenre, MgGenresResp, MgPlatform, MgPlatformsResp,
	MgScreenshotsResp,
};
use anyhow::{Context, anyhow};
use entity::sea_orm_active_enums::MetadataProviderEnum;
use log::debug;
use reqwest::header::HeaderMap;
use reqwest::{Client, Method, StatusCode, Url};
use serde::de::DeserializeOwned;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, OnceCell};
use tower::limit::{RateLimit, RateLimitLayer};
use tower::retry::Retry;
use tower::{Service, ServiceBuilder, ServiceExt};

pub mod cache;
pub mod matching;
pub mod model;

pub const API_URL: &str = "https://api.mobygames.com/v1";

/// Cheapest MobyGames tier permits one request every five seconds.
const RATELIMIT_AMOUNT: u64 = 1;
const RATELIMIT_DURATION_MS: u64 = 5000;

pub struct MobyGamesClient {
	client: Client,
	service: Mutex<RateLimit<Retry<RetryPolicy, Client>>>,
	api_key: String,
	redis_conn: redis::aio::MultiplexedConnection,
	platforms_cache: OnceCell<Arc<Vec<MgPlatform>>>,
	genres_cache: OnceCell<Arc<Vec<MgGenre>>>,
}

impl MobyGamesClient {
	pub fn new(
		api_key: String,
		client: Client,
		redis_conn: redis::aio::MultiplexedConnection,
	) -> anyhow::Result<Self> {
		let rate_limit_layer = RateLimitLayer::new(
			RATELIMIT_AMOUNT,
			Duration::from_millis(RATELIMIT_DURATION_MS),
		);
		let retry_layer = tower::retry::RetryLayer::new(RetryPolicy::new("mobygames"));

		let service = ServiceBuilder::new()
			.layer(rate_limit_layer)
			.layer(retry_layer)
			.service(client.clone());

		crate::metrics::set_provider_concurrency_configured(
			"mobygames",
			crate::providers::DEFAULT_CHUNK_SIZE as i64,
		);

		Ok(Self {
			client,
			service: Mutex::new(service),
			api_key,
			redis_conn,
			platforms_cache: OnceCell::new(),
			genres_cache: OnceCell::new(),
		})
	}

	pub async fn list_platforms(&self) -> anyhow::Result<Arc<Vec<MgPlatform>>> {
		self.platforms_cache
			.get_or_try_init(|| async {
				let url = self.url(&["platforms"], &[])?;
				let resp = self.do_get_url::<MgPlatformsResp>("platforms", url).await?;
				Ok(Arc::new(resp.platforms))
			})
			.await
			.cloned()
	}

	pub async fn list_genres(&self) -> anyhow::Result<Arc<Vec<MgGenre>>> {
		self.genres_cache
			.get_or_try_init(|| async {
				let url = self.url(&["genres"], &[])?;
				let resp = self.do_get_url::<MgGenresResp>("genres", url).await?;
				Ok(Arc::new(resp.genres))
			})
			.await
			.cloned()
	}

	pub async fn search_games(
		&self,
		platform_id: Option<i64>,
		title: &str,
	) -> anyhow::Result<Vec<MgGame>> {
		let mut params: Vec<(&'static str, String)> = vec![
			("title", title.to_string()),
			("format", "normal".to_string()),
		];
		if let Some(pid) = platform_id {
			params.push(("platform", pid.to_string()));
		}
		let url = self.url(&["games"], &params)?;
		let resp = self.do_get_url::<MgGamesResp>("games_search", url).await?;
		Ok(resp.games)
	}

	pub async fn get_game_by_id(&self, game_id: i64) -> anyhow::Result<Option<MgGame>> {
		let url = self.url(&["games", &game_id.to_string()], &[])?;
		self.do_get_optional_url::<MgGame>("games_by_id", url).await
	}

	pub async fn get_game_platform_covers(
		&self,
		game_id: i64,
		platform_id: i64,
	) -> anyhow::Result<MgCoversResp> {
		let url = self.url(
			&[
				"games",
				&game_id.to_string(),
				"platforms",
				&platform_id.to_string(),
				"covers",
			],
			&[],
		)?;
		self.do_get_url::<MgCoversResp>("game_covers", url).await
	}

	pub async fn get_game_platform_screenshots(
		&self,
		game_id: i64,
		platform_id: i64,
	) -> anyhow::Result<MgScreenshotsResp> {
		let url = self.url(
			&[
				"games",
				&game_id.to_string(),
				"platforms",
				&platform_id.to_string(),
				"screenshots",
			],
			&[],
		)?;
		self.do_get_url::<MgScreenshotsResp>("game_screenshots", url)
			.await
	}

	fn url(&self, segments: &[&str], query: &[(&'static str, String)]) -> anyhow::Result<Url> {
		let mut url = Url::parse(API_URL)?;
		url.path_segments_mut()
			.map_err(|_| anyhow!("mobygames base url cannot have path segments"))?
			.extend(segments);
		url.query_pairs_mut().append_pair("api_key", &self.api_key);
		if !query.is_empty() {
			url.query_pairs_mut()
				.extend_pairs(query.iter().map(|(k, v)| (*k, v.as_str())));
		}
		Ok(url)
	}

	async fn do_get_url<T: DeserializeOwned>(
		&self,
		endpoint_label: &'static str,
		url: Url,
	) -> anyhow::Result<T> {
		let started = std::time::Instant::now();
		let _inflight = crate::http::abstraction::InflightGuard::new("mobygames");
		let raw = self.execute_get::<T>(url).await;
		let (status_class, status_code) = match &raw {
			Ok((status, _)) => {
				let code = status.as_u16();
				(
					crate::http::abstraction::classify_status(code),
					code.to_string(),
				)
			}
			Err(_) => ("network_error", "none".to_string()),
		};
		crate::metrics::record_metadata_request(
			"mobygames",
			endpoint_label,
			status_class,
			&status_code,
			started.elapsed().as_secs_f64(),
		);
		raw.and_then(|(status, body)| {
			if !status.is_success() {
				return Err(anyhow!("mobygames returned non-success status: {status}"));
			}
			body.ok_or_else(|| anyhow!("mobygames returned empty body on success"))
		})
	}

	/// 404 maps to `Ok(None)`. Other non-success statuses are errors.
	async fn do_get_optional_url<T: DeserializeOwned>(
		&self,
		endpoint_label: &'static str,
		url: Url,
	) -> anyhow::Result<Option<T>> {
		let started = std::time::Instant::now();
		let _inflight = crate::http::abstraction::InflightGuard::new("mobygames");
		let result = self.execute_get::<T>(url).await;
		let (status_class, status_code) = match &result {
			Ok((status, _)) => {
				let code = status.as_u16();
				(
					crate::http::abstraction::classify_status(code),
					code.to_string(),
				)
			}
			Err(_) => ("network_error", "none".to_string()),
		};
		crate::metrics::record_metadata_request(
			"mobygames",
			endpoint_label,
			status_class,
			&status_code,
			started.elapsed().as_secs_f64(),
		);
		match result? {
			(status, _) if status == StatusCode::NOT_FOUND => Ok(None),
			(status, _) if !status.is_success() => {
				Err(anyhow!("mobygames returned non-success status: {status}"))
			}
			(_, body) => Ok(body),
		}
	}

	async fn execute_get<T: DeserializeOwned>(
		&self,
		url: Url,
	) -> anyhow::Result<(StatusCode, Option<T>)> {
		let mut headers = HeaderMap::new();
		headers.insert("User-Agent", REQWEST_DEFAULT_USER_AGENT.parse()?);
		headers.insert("Accept", "application/json".parse()?);

		let url_for_log = url_for_log(&url);
		let req = self
			.client
			.request(Method::GET, url)
			.headers(headers)
			.build()?;

		debug!("mobygames request: {} {url_for_log}", req.method());

		let rate_limited_future = self.service.lock().await.ready().await?.call(req);
		let res = rate_limited_future.await?;
		let status = res.status();
		let body = res.text().await?;

		if log::log_enabled!(log::Level::Debug) {
			let preview: String = body.chars().take(256).collect();
			debug!("mobygames response (status={status}, first 256): {preview}");
		}

		if status == StatusCode::NOT_FOUND {
			return Ok((status, None));
		}
		if body.is_empty() {
			return Ok((status, None));
		}
		let parsed = serde_json::from_str(&body)
			.with_context(|| format!("failed to parse mobygames response from {url_for_log}"))?;
		Ok((status, Some(parsed)))
	}
}

/// Strips the `api_key` query parameter so log lines do not leak the secret.
fn url_for_log(url: &Url) -> String {
	let mut sanitised = url.clone();
	let pairs: Vec<(String, String)> = sanitised
		.query_pairs()
		.filter(|(k, _)| k != "api_key")
		.map(|(k, v)| (k.into_owned(), v.into_owned()))
		.collect();
	sanitised.set_query(None);
	if !pairs.is_empty() {
		sanitised
			.query_pairs_mut()
			.extend_pairs(pairs.iter().map(|(k, v)| (k.as_str(), v.as_str())));
	}
	sanitised.to_string()
}

#[async_trait::async_trait]
impl crate::providers::MetadataProvider for MobyGamesClient {
	fn provider_label(&self) -> &'static str {
		"mobygames"
	}

	fn provider_enum(&self) -> MetadataProviderEnum {
		MetadataProviderEnum::Mobygames
	}

	fn redis_conn(&self) -> &redis::aio::MultiplexedConnection {
		&self.redis_conn
	}

	async fn match_db(self: Arc<Self>, db_conn: &sea_orm::DbConn) -> anyhow::Result<()> {
		matching::match_db_to_mobygames_entities(self, db_conn).await
	}

	async fn match_via_sibling_names(
		self: Arc<Self>,
		db_conn: &sea_orm::DbConn,
	) -> anyhow::Result<()> {
		crate::providers::drive_cross_match_pipeline(
			"mobygames",
			MetadataProviderEnum::Mobygames,
			matching::game::match_game_via_sibling_name_mobygames,
			self,
			db_conn,
			crate::providers::DEFAULT_CHUNK_SIZE,
		)
		.await
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn url_for_log_strips_api_key() {
		let url = Url::parse(
			"https://api.mobygames.com/v1/games?api_key=supersecret&title=zelda&platform=22",
		)
		.unwrap();
		let logged = url_for_log(&url);
		assert!(!logged.contains("supersecret"), "got: {logged}");
		assert!(logged.contains("title=zelda"));
		assert!(logged.contains("platform=22"));
	}

	#[test]
	fn url_for_log_handles_url_without_query() {
		let url = Url::parse("https://api.mobygames.com/v1/platforms").unwrap();
		let logged = url_for_log(&url);
		assert_eq!(logged, "https://api.mobygames.com/v1/platforms");
	}
}
