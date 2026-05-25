use crate::config::http::REQWEST_DEFAULT_USER_AGENT;
use crate::http::abstraction::RetryPolicy;
use crate::providers::emuready::model::{
	EmuReadyEnvelope, EmuReadyGame, EmuReadyGamesPage, EmuReadySystem,
};
use anyhow::{Context, anyhow};
use entity::sea_orm_active_enums::MetadataProviderEnum;
use log::debug;
use reqwest::header::HeaderMap;
use reqwest::{Client, Method, StatusCode, Url};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, OnceCell};
use tower::limit::{RateLimit, RateLimitLayer};
use tower::retry::Retry;
use tower::{Service, ServiceBuilder, ServiceExt};

pub mod matching;
pub mod model;

pub const API_URL: &str = "https://www.emuready.com/api/mobile/trpc";

const RATELIMIT_AMOUNT: u64 = 4;
const RATELIMIT_DURATION_MS: u64 = 1000;
const SEARCH_LIMIT: i64 = 50;

pub struct EmuReadyClient {
	client: Client,
	service: Mutex<RateLimit<Retry<RetryPolicy, Client>>>,
	redis_conn: redis::aio::MultiplexedConnection,
	systems_cache: OnceCell<Arc<Vec<EmuReadySystem>>>,
}

impl EmuReadyClient {
	pub fn new(
		client: Client,
		redis_conn: redis::aio::MultiplexedConnection,
	) -> anyhow::Result<Self> {
		let rate_limit_layer = RateLimitLayer::new(
			RATELIMIT_AMOUNT,
			Duration::from_millis(RATELIMIT_DURATION_MS),
		);
		let retry_layer = tower::retry::RetryLayer::new(RetryPolicy::new("emuready"));

		let service = ServiceBuilder::new()
			.layer(rate_limit_layer)
			.layer(retry_layer)
			.service(client.clone());

		crate::metrics::set_provider_concurrency_configured(
			"emuready",
			crate::providers::DEFAULT_CHUNK_SIZE as i64,
		);

		Ok(Self {
			client,
			service: Mutex::new(service),
			redis_conn,
			systems_cache: OnceCell::new(),
		})
	}

	pub async fn list_systems(&self) -> anyhow::Result<Arc<Vec<EmuReadySystem>>> {
		self.systems_cache
			.get_or_try_init(|| async {
				let url = build_url("general.systems", None)?;
				let env = self
					.execute_get::<EmuReadyEnvelope<Vec<EmuReadySystem>>>(url, "systems")
					.await?;
				Ok(Arc::new(env.result.data.json))
			})
			.await
			.cloned()
	}

	pub async fn search_games(
		&self,
		system_id: &str,
		query: &str,
	) -> anyhow::Result<Vec<EmuReadyGame>> {
		#[derive(Serialize)]
		struct Input<'a> {
			search: &'a str,
			#[serde(rename = "systemId")]
			system_id: &'a str,
			page: i64,
			limit: i64,
		}
		let payload = Input {
			search: query,
			system_id,
			page: 1,
			limit: SEARCH_LIMIT,
		};
		let input = build_trpc_input(&payload)?;
		let url = build_url("games.get", Some(&input))?;
		let env = self
			.execute_get::<EmuReadyEnvelope<EmuReadyGamesPage>>(url, "games_get")
			.await?;
		Ok(env.result.data.json.games)
	}

	async fn execute_get<T: DeserializeOwned>(
		&self,
		url: Url,
		endpoint_label: &'static str,
	) -> anyhow::Result<T> {
		let mut headers = HeaderMap::new();
		headers.insert("User-Agent", REQWEST_DEFAULT_USER_AGENT.parse()?);
		headers.insert("Accept", "application/json".parse()?);

		let url_for_log = url.clone();
		let req = self
			.client
			.request(Method::GET, url)
			.headers(headers)
			.build()?;

		debug!("emuready request: {} {}", req.method(), url_for_log.path());

		let started = std::time::Instant::now();
		let _inflight = crate::http::abstraction::InflightGuard::new("emuready");
		let mut observed_code: Option<u16> = None;
		let result: anyhow::Result<T> = async {
			let rate_limited_future = self.service.lock().await.ready().await?.call(req);
			let res = rate_limited_future.await?;
			let status = res.status();
			observed_code = Some(status.as_u16());
			let body = res.text().await?;
			if !status.is_success() {
				return Err(anyhow!(
					"emuready returned non-success status {status} from {url_for_log}"
				));
			}
			if status == StatusCode::NO_CONTENT || body.is_empty() {
				return Err(anyhow!(
					"emuready returned empty body on success from {url_for_log}"
				));
			}
			serde_json::from_str::<T>(&body)
				.with_context(|| format!("failed to parse emuready response from {url_for_log}"))
		}
		.await;

		let (status_class, status_code) = match observed_code {
			Some(code) => (
				crate::http::abstraction::classify_status(code),
				code.to_string(),
			),
			None => ("network_error", "none".to_string()),
		};
		crate::metrics::record_metadata_request(
			"emuready",
			endpoint_label,
			status_class,
			&status_code,
			started.elapsed().as_secs_f64(),
		);
		result
	}
}

fn build_url(procedure: &str, input: Option<&str>) -> anyhow::Result<Url> {
	let mut url = Url::parse(API_URL)?;
	url.path_segments_mut()
		.map_err(|_| anyhow!("emuready base url cannot have path segments"))?
		.push(procedure);
	if let Some(input) = input {
		url.query_pairs_mut().append_pair("input", input);
	}
	Ok(url)
}

fn build_trpc_input<T: Serialize>(payload: &T) -> anyhow::Result<String> {
	Ok(serde_json::to_string(
		&serde_json::json!({ "json": payload }),
	)?)
}

#[async_trait::async_trait]
impl crate::providers::MetadataProvider for EmuReadyClient {
	fn provider_label(&self) -> &'static str {
		"emuready"
	}

	fn provider_enum(&self) -> MetadataProviderEnum {
		MetadataProviderEnum::EmuReady
	}

	fn redis_conn(&self) -> &redis::aio::MultiplexedConnection {
		&self.redis_conn
	}

	async fn match_db(self: Arc<Self>, db_conn: &sea_orm::DbConn) -> anyhow::Result<()> {
		matching::match_db_to_emuready_entities(self, db_conn).await
	}

	async fn match_via_sibling_names(
		self: Arc<Self>,
		db_conn: &sea_orm::DbConn,
	) -> anyhow::Result<()> {
		crate::providers::drive_cross_match_pipeline(
			"emuready",
			MetadataProviderEnum::EmuReady,
			matching::game::match_game_via_sibling_name_emuready,
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
	fn build_trpc_input_wraps_payload_in_json_field() {
		#[derive(Serialize)]
		struct P<'a> {
			search: &'a str,
		}
		let s = build_trpc_input(&P { search: "mario" }).unwrap();
		assert_eq!(s, r#"{"json":{"search":"mario"}}"#);
	}

	#[test]
	fn url_for_systems_list_has_no_input_query_param() {
		let url = build_url("general.systems", None).unwrap();
		assert_eq!(url.path(), "/api/mobile/trpc/general.systems");
		assert!(url.query().is_none(), "got: {url}");
	}

	#[test]
	fn url_for_search_includes_url_encoded_input() {
		let input = r#"{"json":{"search":"super mario","systemId":"abc","page":1,"limit":50}}"#;
		let url = build_url("games.get", Some(input)).unwrap();
		assert_eq!(url.path(), "/api/mobile/trpc/games.get");
		let query = url.query().expect("expected ?input=...");
		assert!(query.starts_with("input="), "got: {query}");
		assert!(
			query.contains("super+mario") || query.contains("super%20mario"),
			"expected encoded space in query, got: {query}"
		);
	}
}
