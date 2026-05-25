use crate::config::http::REQWEST_DEFAULT_USER_AGENT;
use crate::http::abstraction::RetryPolicy;
use crate::providers::hasheous::model::{HashLookupRequest, HashLookupResponse};
use anyhow::{Context, anyhow};
use entity::sea_orm_active_enums::MetadataProviderEnum;
use log::debug;
use reqwest::header::HeaderMap;
use reqwest::{Client, Method, StatusCode, Url};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tower::limit::{RateLimit, RateLimitLayer};
use tower::retry::Retry;
use tower::{Service, ServiceBuilder, ServiceExt};

pub mod matching;
pub mod model;

pub const API_URL: &str = "https://hasheous.org/api/v1";

const RATELIMIT_AMOUNT: u64 = 4;
const RATELIMIT_DURATION_MS: u64 = 1000;

pub struct HasheousClient {
	client: Client,
	service: Mutex<RateLimit<Retry<RetryPolicy, Client>>>,
	redis_conn: redis::aio::MultiplexedConnection,
}

pub enum HashLookupOutcome {
	Hit(Box<HashLookupResponse>),
	NotFound,
}

impl HasheousClient {
	pub fn new(
		client: Client,
		redis_conn: redis::aio::MultiplexedConnection,
	) -> anyhow::Result<Self> {
		let rate_limit_layer = RateLimitLayer::new(
			RATELIMIT_AMOUNT,
			Duration::from_millis(RATELIMIT_DURATION_MS),
		);
		let retry_layer = tower::retry::RetryLayer::new(RetryPolicy::new("hasheous"));

		let service = ServiceBuilder::new()
			.layer(rate_limit_layer)
			.layer(retry_layer)
			.service(client.clone());

		Ok(Self {
			client,
			service: Mutex::new(service),
			redis_conn,
		})
	}

	pub async fn lookup_by_hash(
		&self,
		body: &HashLookupRequest,
	) -> anyhow::Result<HashLookupOutcome> {
		let url = build_url("Lookup/ByHash")?;
		self.execute_post(url, body, "lookup_by_hash").await
	}

	async fn execute_post(
		&self,
		url: Url,
		body: &HashLookupRequest,
		endpoint_label: &'static str,
	) -> anyhow::Result<HashLookupOutcome> {
		let mut headers = HeaderMap::new();
		headers.insert("User-Agent", REQWEST_DEFAULT_USER_AGENT.parse()?);
		headers.insert("Accept", "application/json".parse()?);
		headers.insert("Content-Type", "application/json".parse()?);

		let serialised = serde_json::to_vec(body)
			.with_context(|| format!("failed to serialise hasheous {endpoint_label} body"))?;
		let url_for_log = url.clone();
		let req = self
			.client
			.request(Method::POST, url)
			.headers(headers)
			.body(serialised)
			.build()?;

		debug!("hasheous request: {} {}", req.method(), url_for_log.path());

		let started = std::time::Instant::now();
		let mut observed_code: Option<u16> = None;
		let result: anyhow::Result<HashLookupOutcome> = async {
			let inflight = self.service.lock().await.ready().await?.call(req);
			let res = inflight.await?;
			let status = res.status();
			observed_code = Some(status.as_u16());
			if status == StatusCode::NOT_FOUND {
				return Ok(HashLookupOutcome::NotFound);
			}
			let body = res.text().await?;
			if !status.is_success() {
				return Err(anyhow!(
					"hasheous returned non-success status {status} from {url_for_log}"
				));
			}
			if status == StatusCode::NO_CONTENT || body.is_empty() {
				return Ok(HashLookupOutcome::NotFound);
			}
			let parsed: HashLookupResponse = serde_json::from_str(&body)
				.with_context(|| format!("failed to parse hasheous response from {url_for_log}"))?;
			Ok(HashLookupOutcome::Hit(Box::new(parsed)))
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
			"hasheous",
			endpoint_label,
			status_class,
			&status_code,
			started.elapsed().as_secs_f64(),
		);
		result
	}
}

fn build_url(path: &str) -> anyhow::Result<Url> {
	let mut url = Url::parse(API_URL)?;
	url.path_segments_mut()
		.map_err(|_| anyhow!("hasheous base url cannot have path segments"))?
		.extend(path.split('/').filter(|s| !s.is_empty()));
	Ok(url)
}

#[async_trait::async_trait]
impl crate::providers::MetadataProvider for HasheousClient {
	fn provider_label(&self) -> &'static str {
		"hasheous"
	}

	fn provider_enum(&self) -> MetadataProviderEnum {
		MetadataProviderEnum::Hasheous
	}

	fn redis_conn(&self) -> &redis::aio::MultiplexedConnection {
		&self.redis_conn
	}

	fn supports_cross_match(&self) -> bool {
		false
	}

	async fn match_db(self: Arc<Self>, db_conn: &sea_orm::DbConn) -> anyhow::Result<()> {
		matching::match_db_to_hasheous_entities(self, db_conn).await
	}
}
