use crate::config::http::REQWEST_DEFAULT_USER_AGENT;
use crate::http::abstraction::RetryPolicy;
use crate::providers::screenscraper::model::{
	JeuPayload, JeuxPayload, SsEnvelope, SsGame, SsSystem, SsUser, SystemesPayload,
};
use anyhow::{Context, anyhow};
use entity::sea_orm_active_enums::MetadataProviderEnum;
use log::{debug, warn};
use reqwest::header::HeaderMap;
use reqwest::{Client, Method, StatusCode, Url};
use serde::de::DeserializeOwned;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::{Mutex, OnceCell};
use tower::limit::{RateLimit, RateLimitLayer};
use tower::retry::Retry;
use tower::{Service, ServiceBuilder, ServiceExt};

pub mod cache;
pub mod matching;
pub mod model;

pub const API_URL: &str = "https://api.screenscraper.fr/api2";

/// Identifies our integration to ScreenScraper. Required by the API; bare
/// `devid`/`devpassword` calls without `softname` get rejected.
const SOFTNAME: &str = "playmatch";

const RATELIMIT_AMOUNT: u64 = 1;
const RATELIMIT_DURATION_MS: u64 = 1200;
const MAX_RETRIES: usize = 3;

/// At or above this fraction of the daily request budget we stop the cycle
/// early so the cron resumes after the daily reset rather than burning the
/// last few requests on partial work.
const QUOTA_SOFT_LIMIT_NUMERATOR: u64 = 95;
const QUOTA_SOFT_LIMIT_DENOMINATOR: u64 = 100;

/// French phrases ScreenScraper returns as plain text (or embedded in HTML)
/// when the API is overloaded or rejecting traffic. Sniffed before we try to
/// parse a body as JSON because incident pages come back with HTTP 200.
/// Sourced from the Skyscraper project.
const INCIDENT_PHRASES: &[&str] = &[
	"non trouvée",
	"API totalement fermé",
	"blacklisté",
	"Votre quota de scrape est",
	"API fermé pour les non membres",
	"maximum threads",
];

pub struct ScreenScraperClient {
	client: Client,
	service: Mutex<RateLimit<Retry<RetryPolicy, Client>>>,
	dev_id: String,
	dev_password: String,
	user: Option<(String, String)>,
	quota_exhausted: AtomicBool,
	systems_cache: OnceCell<Arc<Vec<SsSystem>>>,
}

impl ScreenScraperClient {
	pub fn new(
		dev_id: String,
		dev_password: String,
		user: Option<(String, String)>,
		client: Client,
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
			service: Mutex::new(service),
			dev_id,
			dev_password,
			user,
			quota_exhausted: AtomicBool::new(false),
			systems_cache: OnceCell::new(),
		})
	}

	pub fn is_quota_exhausted(&self) -> bool {
		self.quota_exhausted.load(Ordering::Relaxed)
	}

	pub async fn list_systems(&self) -> anyhow::Result<Arc<Vec<SsSystem>>> {
		let cell = &self.systems_cache;
		cell.get_or_try_init(|| async {
			let url = self.url("systemesListe.php", &[])?;
			let env = self
				.do_get_envelope::<SystemesPayload>("systems_list", url)
				.await?;
			let systems = env.response.map(|r| r.payload.systemes).unwrap_or_default();
			Ok(Arc::new(systems))
		})
		.await
		.cloned()
	}

	pub async fn search_games(&self, system_id: i32, term: &str) -> anyhow::Result<Vec<SsGame>> {
		let url = self.url(
			"jeuRecherche.php",
			&[
				("systemeid", system_id.to_string()),
				("recherche", term.to_string()),
			],
		)?;
		let env = self
			.do_get_envelope::<JeuxPayload>("game_search", url)
			.await?;
		Ok(env.response.map(|r| r.payload.jeux).unwrap_or_default())
	}

	pub async fn get_game_by_id(&self, game_id: i64) -> anyhow::Result<Option<SsGame>> {
		let url = self.url("jeuInfos.php", &[("gameid", game_id.to_string())])?;
		self.fetch_optional_game("game_by_id", url).await
	}

	pub async fn get_game_by_rom_name(
		&self,
		system_id: i32,
		rom_name: &str,
	) -> anyhow::Result<Option<SsGame>> {
		let url = self.url(
			"jeuInfos.php",
			&[
				("systemeid", system_id.to_string()),
				("romnom", rom_name.to_string()),
			],
		)?;
		self.fetch_optional_game("game_by_rom", url).await
	}

	pub async fn get_game_by_md5(
		&self,
		system_id: i32,
		md5: &str,
	) -> anyhow::Result<Option<SsGame>> {
		let url = self.url(
			"jeuInfos.php",
			&[
				("systemeid", system_id.to_string()),
				("md5", md5.to_string()),
			],
		)?;
		self.fetch_optional_game("game_by_md5", url).await
	}

	pub async fn get_game_by_sha1(
		&self,
		system_id: i32,
		sha1: &str,
	) -> anyhow::Result<Option<SsGame>> {
		let url = self.url(
			"jeuInfos.php",
			&[
				("systemeid", system_id.to_string()),
				("sha1", sha1.to_string()),
			],
		)?;
		self.fetch_optional_game("game_by_sha1", url).await
	}

	pub async fn get_game_by_crc(
		&self,
		system_id: i32,
		crc: &str,
	) -> anyhow::Result<Option<SsGame>> {
		let url = self.url(
			"jeuInfos.php",
			&[
				("systemeid", system_id.to_string()),
				("crc", crc.to_string()),
			],
		)?;
		self.fetch_optional_game("game_by_crc", url).await
	}

	async fn fetch_optional_game(
		&self,
		endpoint_label: &'static str,
		url: Url,
	) -> anyhow::Result<Option<SsGame>> {
		match self
			.do_get_envelope_optional::<JeuPayload>(endpoint_label, url)
			.await?
		{
			Some(env) => Ok(env.response.map(|r| r.payload.jeu)),
			None => Ok(None),
		}
	}

	fn auth_query_pairs(&self) -> Vec<(&'static str, String)> {
		let mut pairs: Vec<(&'static str, String)> = vec![
			("devid", self.dev_id.clone()),
			("devpassword", self.dev_password.clone()),
			("softname", SOFTNAME.to_string()),
			("output", "json".to_string()),
		];
		if let Some((id, pw)) = &self.user {
			pairs.push(("ssid", id.clone()));
			pairs.push(("sspassword", pw.clone()));
		}
		pairs
	}

	fn url(&self, endpoint: &str, extra: &[(&'static str, String)]) -> anyhow::Result<Url> {
		let mut url = Url::parse(API_URL)?;
		url.path_segments_mut()
			.map_err(|_| anyhow!("screenscraper base url cannot have path segments"))?
			.push(endpoint);
		let mut pairs = self.auth_query_pairs();
		pairs.extend(extra.iter().map(|(k, v)| (*k, v.clone())));
		url.query_pairs_mut()
			.extend_pairs(pairs.iter().map(|(k, v)| (*k, v.as_str())));
		Ok(url)
	}

	async fn do_get_envelope<T: DeserializeOwned>(
		&self,
		endpoint_label: &'static str,
		url: Url,
	) -> anyhow::Result<SsEnvelope<T>> {
		match self
			.do_get_envelope_optional::<T>(endpoint_label, url)
			.await?
		{
			Some(env) => Ok(env),
			None => Err(anyhow!("screenscraper returned no envelope")),
		}
	}

	/// 404 maps to `Ok(None)`. Other non-success statuses are errors. Updates
	/// `quota_exhausted` from the envelope's `ssuser` block on success.
	async fn do_get_envelope_optional<T: DeserializeOwned>(
		&self,
		endpoint_label: &'static str,
		url: Url,
	) -> anyhow::Result<Option<SsEnvelope<T>>> {
		let started = std::time::Instant::now();
		let result = self.execute_get(url).await;
		let outcome = match &result {
			Ok((status, _, _)) if status.is_success() => "success",
			Ok((status, _, _)) if *status == StatusCode::NOT_FOUND => "not_found",
			Ok(_) => "error",
			Err(_) => "error",
		};
		crate::metrics::record_metadata_request(
			"screenscraper",
			endpoint_label,
			outcome,
			started.elapsed().as_secs_f64(),
		);

		let (status, content_type, body) = result?;

		match status {
			s if s == StatusCode::NOT_FOUND => Ok(None),
			s if s.as_u16() == 430 => {
				self.quota_exhausted.store(true, Ordering::Relaxed);
				Err(anyhow!(
					"screenscraper daily quota exhausted (HTTP 430), aborting cycle"
				))
			}
			s if matches!(s.as_u16(), 401 | 426 | 429 | 431) => {
				self.quota_exhausted.store(true, Ordering::Relaxed);
				Err(anyhow!(
					"screenscraper rejected the request with HTTP {s} (server overloaded or thread limit), aborting cycle"
				))
			}
			s if !s.is_success() => Err(anyhow!("screenscraper returned non-success status: {s}")),
			_ if body.is_empty() => Ok(None),
			_ => {
				parse_or_incident(&body, content_type.as_deref())?;
				let env: SsEnvelope<T> = serde_json::from_str(&body)
					.with_context(|| "failed to parse screenscraper response envelope")?;
				let header_signals_failure = env
					.header
					.as_ref()
					.is_some_and(|h| h.success.eq_ignore_ascii_case("false"));
				if let Some(resp) = env.response.as_ref() {
					self.update_quota_from(&resp.ssuser);
				}
				if header_signals_failure {
					return Err(anyhow!(
						"screenscraper header reported failure: {:?}",
						env.header.and_then(|h| h.error)
					));
				}
				Ok(Some(env))
			}
		}
	}

	async fn execute_get(&self, url: Url) -> anyhow::Result<(StatusCode, Option<String>, String)> {
		let mut headers = HeaderMap::new();
		headers.insert("User-Agent", REQWEST_DEFAULT_USER_AGENT.parse()?);
		headers.insert("Accept", "application/json,text/plain;q=0.5".parse()?);

		let url_for_log = url_for_log(&url);
		let req = self
			.client
			.request(Method::GET, url)
			.headers(headers)
			.build()?;

		debug!("screenscraper request: {} {url_for_log}", req.method());

		let rate_limited_future = self.service.lock().await.ready().await?.call(req);
		let res = rate_limited_future.await?;
		let status = res.status();
		let content_type = res
			.headers()
			.get(reqwest::header::CONTENT_TYPE)
			.and_then(|v| v.to_str().ok())
			.map(|s| s.to_string());
		let body = res.text().await?;

		if log::log_enabled!(log::Level::Debug) {
			let preview: String = body.chars().take(256).collect();
			debug!("screenscraper response (status={status}, first 256): {preview}");
		}

		Ok((status, content_type, body))
	}

	fn update_quota_from(&self, user: &Option<SsUser>) {
		let Some(user) = user else { return };
		let Some(today) = user
			.requeststoday
			.as_deref()
			.and_then(|s| s.parse::<u64>().ok())
		else {
			return;
		};
		let Some(max) = user
			.maxrequestsperday
			.as_deref()
			.and_then(|s| s.parse::<u64>().ok())
		else {
			return;
		};
		if max == 0 {
			return;
		}
		if today * QUOTA_SOFT_LIMIT_DENOMINATOR >= max * QUOTA_SOFT_LIMIT_NUMERATOR {
			if !self.quota_exhausted.swap(true, Ordering::Relaxed) {
				warn!(
					"screenscraper quota near limit ({today}/{max}); short-circuiting remaining match cycle"
				);
			}
		}
	}
}

/// Builds a logging-safe URL by stripping password query parameters. Never
/// log the raw URL because `devpassword` and `sspassword` ride in the query
/// string on every request.
fn url_for_log(url: &Url) -> String {
	let mut sanitised = url.clone();
	let pairs: Vec<(String, String)> = sanitised
		.query_pairs()
		.filter(|(k, _)| k != "devpassword" && k != "sspassword")
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

/// Returns `Err` when the body looks like a French incident message or the
/// content type is not JSON, so the caller does not try to parse the body and
/// the per-game match logs an error instead of aborting the whole cycle.
fn parse_or_incident(body: &str, content_type: Option<&str>) -> anyhow::Result<()> {
	if let Some(ct) = content_type
		&& !ct.to_ascii_lowercase().contains("application/json")
	{
		return Err(anyhow!(
			"screenscraper returned non-json content-type {ct}, treating as incident"
		));
	}

	let prefix: String = body.chars().take(1024).collect();
	let prefix_lower = prefix.to_lowercase();
	for phrase in INCIDENT_PHRASES {
		if prefix_lower.contains(&phrase.to_lowercase()) {
			return Err(anyhow!(
				"screenscraper response matched incident phrase: {phrase}"
			));
		}
	}

	Ok(())
}

#[async_trait::async_trait]
impl crate::providers::MetadataProvider for ScreenScraperClient {
	fn provider_label(&self) -> &'static str {
		"screenscraper"
	}

	fn provider_enum(&self) -> MetadataProviderEnum {
		MetadataProviderEnum::Screenscraper
	}

	fn chunk_size(&self) -> usize {
		1
	}

	async fn match_db(self: Arc<Self>, db_conn: &sea_orm::DbConn) -> anyhow::Result<()> {
		if self.is_quota_exhausted() {
			warn!("screenscraper quota exhausted at cycle start, skipping");
			return Ok(());
		}
		matching::match_db_to_screenscraper_entities(self, db_conn).await
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use reqwest::Client;

	fn client() -> ScreenScraperClient {
		ScreenScraperClient::new(
			"dev".into(),
			"devpw".into(),
			Some(("user".into(), "userpw".into())),
			Client::new(),
		)
		.unwrap()
	}

	#[test]
	fn parse_or_incident_rejects_each_french_phrase() {
		for phrase in INCIDENT_PHRASES {
			let body = format!("preamble {phrase} trailing");
			assert!(
				parse_or_incident(&body, Some("application/json")).is_err(),
				"phrase {phrase} should be flagged"
			);
		}
	}

	#[test]
	fn parse_or_incident_rejects_non_json_content_type() {
		let body = "{\"jeu\": {\"id\": \"1\"}}";
		assert!(parse_or_incident(body, Some("text/html; charset=utf-8")).is_err());
	}

	#[test]
	fn parse_or_incident_accepts_valid_json_body() {
		let body = "{\"header\": {\"success\": \"true\"}}";
		assert!(parse_or_incident(body, Some("application/json")).is_ok());
	}

	#[test]
	fn url_for_log_strips_passwords() {
		let url = Url::parse(
			"https://api.screenscraper.fr/api2/jeuInfos.php?devid=foo&devpassword=secret&ssid=u&sspassword=topsecret&gameid=42",
		)
		.unwrap();
		let logged = url_for_log(&url);
		assert!(!logged.contains("secret"), "got: {logged}");
		assert!(!logged.contains("topsecret"), "got: {logged}");
		assert!(logged.contains("devid=foo"));
		assert!(logged.contains("gameid=42"));
	}

	#[tokio::test]
	async fn update_quota_flips_at_or_above_95_percent() {
		let c = client();
		let user = Some(SsUser {
			requeststoday: Some("950".into()),
			maxrequestsperday: Some("1000".into()),
			maxrequestspermin: None,
			maxthreads: None,
		});
		c.update_quota_from(&user);
		assert!(c.is_quota_exhausted());
	}

	#[tokio::test]
	async fn update_quota_does_not_flip_below_95_percent() {
		let c = client();
		let user = Some(SsUser {
			requeststoday: Some("900".into()),
			maxrequestsperday: Some("1000".into()),
			maxrequestspermin: None,
			maxthreads: None,
		});
		c.update_quota_from(&user);
		assert!(!c.is_quota_exhausted());
	}
}
