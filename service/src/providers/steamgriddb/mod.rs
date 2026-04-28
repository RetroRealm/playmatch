use crate::config::http::REQWEST_DEFAULT_USER_AGENT;
use crate::http::abstraction::RetryPolicy;
use crate::providers::steamgriddb::model::{
	AssetFilters, SgdbAsset, SgdbGame, SgdbListEnvelope, SgdbPlatform, SgdbSingleEnvelope,
};
use anyhow::{Context, anyhow};
use entity::sea_orm_active_enums::MetadataProviderEnum;
use log::debug;
use reqwest::header::HeaderMap;
use reqwest::{Client, Method, StatusCode, Url};
use serde::de::DeserializeOwned;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tower::limit::{RateLimit, RateLimitLayer};
use tower::retry::Retry;
use tower::{Service, ServiceBuilder, ServiceExt};

pub mod cache;
pub mod matching;
pub mod model;

pub const API_URL: &str = "https://www.steamgriddb.com/api/v2";

const RATELIMIT_AMOUNT: u64 = 8;
const RATELIMIT_DURATION_MS: u64 = 1000;
const MAX_RETRIES: usize = 3;

pub struct SteamGridDbClient {
	client: Client,
	service: Mutex<RateLimit<Retry<RetryPolicy, Client>>>,
	bearer: String,
	redis_conn: redis::aio::MultiplexedConnection,
}

impl SteamGridDbClient {
	pub fn new(
		bearer: String,
		client: Client,
		redis_conn: redis::aio::MultiplexedConnection,
	) -> anyhow::Result<Self> {
		let rate_limit_layer = RateLimitLayer::new(
			RATELIMIT_AMOUNT,
			Duration::from_millis(RATELIMIT_DURATION_MS),
		);
		let retry_layer = tower::retry::RetryLayer::new(RetryPolicy(MAX_RETRIES));

		let service = ServiceBuilder::new()
			.layer(rate_limit_layer)
			.layer(retry_layer)
			.service(client.clone());

		Ok(Self {
			client,
			bearer,
			service: Mutex::new(service),
			redis_conn,
		})
	}

	pub fn redis_conn(&self) -> &redis::aio::MultiplexedConnection {
		&self.redis_conn
	}

	pub async fn get_game_by_id(&self, id: i64) -> anyhow::Result<Option<SgdbGame>> {
		let url = self.url(&["games", "id", &id.to_string()], &[])?;
		match self
			.do_get_optional_url::<SgdbSingleEnvelope<SgdbGame>>("games_by_id", url)
			.await?
		{
			Some(env) if env.success => Ok(env.data),
			_ => Ok(None),
		}
	}

	pub async fn get_game_by_platform(
		&self,
		platform: SgdbPlatform,
		platform_id: &str,
	) -> anyhow::Result<Option<SgdbGame>> {
		let url = self.url(&["games", platform.as_str(), platform_id], &[])?;
		match self
			.do_get_optional_url::<SgdbSingleEnvelope<SgdbGame>>("games_by_platform", url)
			.await?
		{
			Some(env) if env.success => Ok(env.data),
			_ => Ok(None),
		}
	}

	pub async fn search_games(&self, term: &str) -> anyhow::Result<Vec<SgdbGame>> {
		// `Url::path_segments_mut` percent-encodes user input safely; a raw
		// `format!` would break on spaces and reserved characters in the term.
		let mut url = Url::parse(API_URL)?;
		url.path_segments_mut()
			.map_err(|_| anyhow!("steamgriddb base url cannot have path segments"))?
			.extend(&["search", "autocomplete", term]);
		let env = self
			.do_get_url::<SgdbListEnvelope<SgdbGame>>("search_autocomplete", url)
			.await?;
		Ok(if env.success { env.data } else { Vec::new() })
	}

	pub async fn get_grids_by_game(
		&self,
		game_id: i64,
		filters: &AssetFilters,
	) -> anyhow::Result<Vec<SgdbAsset>> {
		self.get_assets_by_game("grids_by_game", "grids", game_id, filters)
			.await
	}

	pub async fn get_grids_by_platform(
		&self,
		platform: SgdbPlatform,
		platform_id: &str,
		filters: &AssetFilters,
	) -> anyhow::Result<Vec<SgdbAsset>> {
		self.get_assets_by_platform("grids_by_platform", "grids", platform, platform_id, filters)
			.await
	}

	pub async fn get_heroes_by_game(
		&self,
		game_id: i64,
		filters: &AssetFilters,
	) -> anyhow::Result<Vec<SgdbAsset>> {
		self.get_assets_by_game("heroes_by_game", "heroes", game_id, filters)
			.await
	}

	pub async fn get_heroes_by_platform(
		&self,
		platform: SgdbPlatform,
		platform_id: &str,
		filters: &AssetFilters,
	) -> anyhow::Result<Vec<SgdbAsset>> {
		self.get_assets_by_platform(
			"heroes_by_platform",
			"heroes",
			platform,
			platform_id,
			filters,
		)
		.await
	}

	pub async fn get_logos_by_game(
		&self,
		game_id: i64,
		filters: &AssetFilters,
	) -> anyhow::Result<Vec<SgdbAsset>> {
		self.get_assets_by_game("logos_by_game", "logos", game_id, filters)
			.await
	}

	pub async fn get_logos_by_platform(
		&self,
		platform: SgdbPlatform,
		platform_id: &str,
		filters: &AssetFilters,
	) -> anyhow::Result<Vec<SgdbAsset>> {
		self.get_assets_by_platform("logos_by_platform", "logos", platform, platform_id, filters)
			.await
	}

	pub async fn get_icons_by_game(
		&self,
		game_id: i64,
		filters: &AssetFilters,
	) -> anyhow::Result<Vec<SgdbAsset>> {
		self.get_assets_by_game("icons_by_game", "icons", game_id, filters)
			.await
	}

	pub async fn get_icons_by_platform(
		&self,
		platform: SgdbPlatform,
		platform_id: &str,
		filters: &AssetFilters,
	) -> anyhow::Result<Vec<SgdbAsset>> {
		self.get_assets_by_platform("icons_by_platform", "icons", platform, platform_id, filters)
			.await
	}

	async fn get_assets_by_game(
		&self,
		endpoint_label: &'static str,
		asset_kind: &str,
		game_id: i64,
		filters: &AssetFilters,
	) -> anyhow::Result<Vec<SgdbAsset>> {
		let url = self.url(
			&[asset_kind, "game", &game_id.to_string()],
			&filters.to_query_pairs(),
		)?;
		let env = self
			.do_get_url::<SgdbListEnvelope<SgdbAsset>>(endpoint_label, url)
			.await?;
		Ok(if env.success { env.data } else { Vec::new() })
	}

	async fn get_assets_by_platform(
		&self,
		endpoint_label: &'static str,
		asset_kind: &str,
		platform: SgdbPlatform,
		platform_id: &str,
		filters: &AssetFilters,
	) -> anyhow::Result<Vec<SgdbAsset>> {
		let url = self.url(
			&[asset_kind, platform.as_str(), platform_id],
			&filters.to_query_pairs(),
		)?;
		let env = self
			.do_get_url::<SgdbListEnvelope<SgdbAsset>>(endpoint_label, url)
			.await?;
		Ok(if env.success { env.data } else { Vec::new() })
	}

	fn url(&self, segments: &[&str], query: &[(&'static str, String)]) -> anyhow::Result<Url> {
		let mut url = Url::parse(API_URL)?;
		url.path_segments_mut()
			.map_err(|_| anyhow!("steamgriddb base url cannot have path segments"))?
			.extend(segments);
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
		let result = self.execute_get::<T>(url).await.and_then(|(status, body)| {
			if !status.is_success() {
				return Err(anyhow!("steamgriddb returned non-success status: {status}"));
			}
			body.ok_or_else(|| anyhow!("steamgriddb returned empty body on success"))
		});
		let outcome = if result.is_ok() { "success" } else { "error" };
		crate::metrics::record_metadata_request(
			"steamgriddb",
			endpoint_label,
			outcome,
			started.elapsed().as_secs_f64(),
		);
		result
	}

	/// 404 maps to `Ok(None)`. Other non-success statuses are errors.
	async fn do_get_optional_url<T: DeserializeOwned>(
		&self,
		endpoint_label: &'static str,
		url: Url,
	) -> anyhow::Result<Option<T>> {
		let started = std::time::Instant::now();
		let result = self.execute_get::<T>(url).await;
		let outcome = match &result {
			Ok((status, _)) if status.is_success() => "success",
			Ok((status, _)) if *status == StatusCode::NOT_FOUND => "not_found",
			Ok(_) => "error",
			Err(_) => "error",
		};
		crate::metrics::record_metadata_request(
			"steamgriddb",
			endpoint_label,
			outcome,
			started.elapsed().as_secs_f64(),
		);
		match result? {
			(status, _) if status == StatusCode::NOT_FOUND => Ok(None),
			(status, _) if !status.is_success() => {
				Err(anyhow!("steamgriddb returned non-success status: {status}"))
			}
			(_, body) => Ok(body),
		}
	}

	async fn execute_get<T: DeserializeOwned>(
		&self,
		url: Url,
	) -> anyhow::Result<(StatusCode, Option<T>)> {
		let mut headers = HeaderMap::new();
		headers.insert(
			"Authorization",
			format!("Bearer {}", self.bearer)
				.parse()
				.context("invalid bearer token characters")?,
		);
		headers.insert("User-Agent", REQWEST_DEFAULT_USER_AGENT.parse()?);
		headers.insert("Accept", "application/json".parse()?);

		let url_for_log = url.clone();
		let req = self
			.client
			.request(Method::GET, url)
			.headers(headers)
			.build()?;

		debug!(
			"steamgriddb request: {} {}",
			req.method(),
			url_for_log.path()
		);

		let rate_limited_future = self.service.lock().await.ready().await?.call(req);
		let res = rate_limited_future.await?;
		let status = res.status();
		let body = res.text().await?;

		if log::log_enabled!(log::Level::Debug) {
			let preview: String = body.chars().take(256).collect();
			debug!("steamgriddb response (status={status}, first 256): {preview}");
		}

		if status == StatusCode::NOT_FOUND {
			return Ok((status, None));
		}
		if body.is_empty() {
			return Ok((status, None));
		}
		let parsed = serde_json::from_str(&body)
			.with_context(|| format!("failed to parse steamgriddb response from {url_for_log}"))?;
		Ok((status, Some(parsed)))
	}
}

#[async_trait::async_trait]
impl crate::providers::MetadataProvider for SteamGridDbClient {
	fn provider_label(&self) -> &'static str {
		"steamgriddb"
	}

	fn provider_enum(&self) -> MetadataProviderEnum {
		MetadataProviderEnum::Steamgriddb
	}

	async fn match_db(self: Arc<Self>, db_conn: &sea_orm::DbConn) -> anyhow::Result<()> {
		matching::match_steamgriddb_to_db_games(self, db_conn).await
	}

	async fn match_via_sibling_names(
		self: Arc<Self>,
		db_conn: &sea_orm::DbConn,
	) -> anyhow::Result<()> {
		crate::providers::drive_cross_match_pipeline(
			"steamgriddb",
			MetadataProviderEnum::Steamgriddb,
			matching::game::match_game_via_sibling_name_steamgriddb,
			self,
			db_conn,
			crate::providers::DEFAULT_CHUNK_SIZE,
		)
		.await
	}
}
