use crate::config::http::REQWEST_DEFAULT_USER_AGENT;
use crate::http::abstraction::RetryPolicy;
use crate::providers::screenscraper::model::{
	JeuPayload, JeuxPayload, SsEnvelope, SsGame, SsSystem, SsUser, SystemesPayload,
};
use anyhow::{Context, anyhow};
use chrono::{Duration as ChronoDuration, TimeZone, Utc};
use chrono_tz::Europe::Paris;
use entity::sea_orm_active_enums::MetadataProviderEnum;
use log::{debug, error, info, warn};
use rand::RngExt;
use redis::AsyncCommands;
use reqwest::header::HeaderMap;
use reqwest::{Client, Method, StatusCode, Url};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, OnceCell, Semaphore};
use tokio::time::sleep;
use tower::retry::Retry;
use tower::{Service, ServiceBuilder, ServiceExt};

pub mod cache;
pub mod matching;
pub mod model;

pub const API_URL: &str = "https://api.screenscraper.fr/api2";

/// Identifies our integration to ScreenScraper. Required by the API; bare
/// `devid`/`devpassword` calls without `softname` get rejected.
const SOFTNAME: &str = "playmatch";

/// Per-thread courtesy interval after every request. Skyscraper's maintainer
/// recommends ~1.2s between calls so ScreenScraper does not flag the client
/// as abusive; we apply it per-permit so concurrent threads each pace
/// themselves rather than sharing a single global token bucket.
const POST_REQUEST_DELAY_MS: u64 = 1200;

/// Hard ceiling on the concurrency probed from `ssuser.maxthreads`.
const MAX_CONCURRENCY: usize = 16;

/// At or above this fraction of the daily request budget we stop the cycle
/// early so the cron resumes after the daily reset rather than burning the
/// last few requests on partial work.
const QUOTA_SOFT_LIMIT_NUMERATOR: u64 = 95;
const QUOTA_SOFT_LIMIT_DENOMINATOR: u64 = 100;

/// Retries for HTTP 429 thread-cap responses on the same credentials. 429
/// does not charge a quota request; it just means too many in flight right
/// now. Backoff entries are milliseconds, jitter added per attempt.
const MAX_429_RETRIES: usize = 3;
const RETRY_BACKOFF_MS: &[u64] = &[250, 500, 1000];

/// HTTP 423 indicates the upstream API is fully closed. Suppress further
/// traffic for this window so we do not pile retries on a server that just
/// told us to back off.
const OUTAGE_BLOCK_SECS: i64 = 300;

const REDIS_KEY_PREFIX: &str = "playmatch:screenscraper:account:";

/// French phrases ScreenScraper returns as plain text (or embedded in HTML)
/// when the API is overloaded or rejecting traffic. Sniffed before we try to
/// parse a body as JSON because incident pages come back with HTTP 200.
/// Sourced from the Skyscraper project.
const INCIDENT_PHRASES: &[&str] = &[
	"API totalement fermé",
	"blacklisté",
	"Votre quota de scrape est",
	"API fermé pour les non membres",
	"maximum threads",
];

/// Body fragments ScreenScraper returns on a per-request miss. Routed to
/// `Ok(None)` so the matcher records a miss instead of bailing.
const NOT_FOUND_PHRASES: &[&str] = &["non trouvée"];

struct Account {
	id: String,
	password: String,
	exhausted_until_unix: AtomicI64,
	last_used_at_unix: AtomicI64,
	concurrency: AtomicUsize,
	permits: Arc<Semaphore>,
	redis_key: String,
	quota_redis_key: String,
}

impl Account {
	fn new(id: String, password: String) -> Self {
		let hashed = sha256_hex(&id);
		let redis_key = format!("{REDIS_KEY_PREFIX}{hashed}:exhausted_until");
		let quota_redis_key = format!("{REDIS_KEY_PREFIX}{hashed}:quota");
		Self {
			id,
			password,
			exhausted_until_unix: AtomicI64::new(0),
			last_used_at_unix: AtomicI64::new(0),
			concurrency: AtomicUsize::new(1),
			permits: Arc::new(Semaphore::new(1)),
			redis_key,
			quota_redis_key,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct QuotaSnapshot {
	requests_today: u64,
	max_requests_per_day: u64,
	requests_ko_today: u64,
	max_requests_ko_per_day: u64,
}

pub struct ScreenScraperClient {
	client: Client,
	service: Mutex<Retry<RetryPolicy, Client>>,
	dev_id: String,
	dev_password: String,
	account: Option<Arc<Account>>,
	outage_until_unix: AtomicI64,
	blacklisted: AtomicBool,
	systems_cache: OnceCell<Arc<Vec<SsSystem>>>,
	redis_conn: redis::aio::MultiplexedConnection,
}

impl ScreenScraperClient {
	pub fn new(
		dev_id: String,
		dev_password: String,
		user: Option<(String, String)>,
		client: Client,
		redis_conn: redis::aio::MultiplexedConnection,
	) -> anyhow::Result<Self> {
		let retry_layer = tower::retry::RetryLayer::new(RetryPolicy::new("screenscraper"));

		let service = ServiceBuilder::new()
			.layer(retry_layer)
			.service(client.clone());

		let account = user.map(|(id, pw)| Arc::new(Account::new(id, pw)));

		crate::metrics::set_screenscraper_concurrency(if account.is_some() { 1 } else { 0 });
		Ok(Self {
			client,
			service: Mutex::new(service),
			dev_id,
			dev_password,
			account,
			outage_until_unix: AtomicI64::new(0),
			blacklisted: AtomicBool::new(false),
			systems_cache: OnceCell::new(),
			redis_conn,
		})
	}

	pub fn secs_until_recovery(&self) -> Option<u64> {
		let pool_earliest: Vec<i64> = self
			.account
			.as_ref()
			.map(|a| vec![a.exhausted_until_unix.load(Ordering::Relaxed)])
			.unwrap_or_default();
		compute_secs_until_recovery(
			self.blacklisted.load(Ordering::Relaxed),
			self.outage_until_unix.load(Ordering::Relaxed),
			&pool_earliest,
			now_unix_secs(),
		)
	}

	pub fn is_quota_exhausted(&self) -> bool {
		if self.blacklisted.load(Ordering::Relaxed) {
			return true;
		}
		let now = now_unix_secs();
		if self.outage_until_unix.load(Ordering::Relaxed) > now {
			return true;
		}
		match self.account.as_ref() {
			Some(a) => a.exhausted_until_unix.load(Ordering::Relaxed) > now,
			None => false,
		}
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
		if !valid_system_id(system_id) {
			debug!("screenscraper search_games skipped: invalid system_id ({system_id})");
			return Ok(vec![]);
		}
		// ScreenScraper's jeuRecherche.php rejects searches with fewer than
		// three meaningful characters with HTTP 400 ("Il manque des champs
		// obligatoires dans l'url"), so skip the round-trip entirely.
		let trimmed = term.trim();
		if trimmed.chars().filter(|c| c.is_alphanumeric()).count() < 3 {
			debug!("screenscraper search_games skipped: term too short ({term:?})");
			return Ok(vec![]);
		}
		let url = self.url(
			"jeuRecherche.php",
			&[
				("systemeid", system_id.to_string()),
				("recherche", trimmed.to_string()),
			],
		)?;
		// `header.success="false"` on jeuRecherche.php is "no matches", not an
		// error; treat as an empty page so the matcher's bottom
		// `write_auto_match_failed` still fires.
		let env = self
			.do_get_envelope_optional::<JeuxPayload>("game_search", url)
			.await?;
		Ok(env
			.and_then(|env| env.response.map(|r| r.payload.jeux))
			.unwrap_or_default())
	}

	pub async fn get_game_by_id(&self, game_id: i64) -> anyhow::Result<Option<SsGame>> {
		if game_id <= 0 {
			debug!("screenscraper get_game_by_id skipped: invalid game_id ({game_id})");
			return Ok(None);
		}
		let url = self.url("jeuInfos.php", &[("gameid", game_id.to_string())])?;
		self.fetch_optional_game("game_by_id", url).await
	}

	pub async fn get_game_by_rom_name(
		&self,
		system_id: i32,
		rom_name: &str,
	) -> anyhow::Result<Option<SsGame>> {
		if !valid_system_id(system_id) {
			debug!("screenscraper get_game_by_rom_name skipped: invalid system_id ({system_id})");
			return Ok(None);
		}
		let trimmed = rom_name.trim();
		if trimmed.is_empty() {
			debug!("screenscraper get_game_by_rom_name skipped: empty rom_name");
			return Ok(None);
		}
		let url = self.url(
			"jeuInfos.php",
			&[
				("systemeid", system_id.to_string()),
				("romnom", trimmed.to_string()),
			],
		)?;
		self.fetch_optional_game("game_by_rom", url).await
	}

	/// Submits `sha1`, `md5` and `crc` in one `jeuInfos.php` call rather than
	/// three sequential calls. Halves outbound traffic per file, which matters
	/// under the per-day-KO ceiling.
	pub async fn get_game_by_hashes(
		&self,
		system_id: i32,
		rom_name: &str,
		rom_size: Option<i64>,
		md5: Option<&str>,
		sha1: Option<&str>,
		crc: Option<&str>,
	) -> anyhow::Result<Option<SsGame>> {
		const ENDPOINT_LABEL: &str = "game_by_hashes";
		if !valid_system_id(system_id) {
			debug!("screenscraper {ENDPOINT_LABEL} skipped: invalid system_id ({system_id})");
			return Ok(None);
		}
		let trimmed_name = rom_name.trim();
		if trimmed_name.is_empty() {
			debug!("screenscraper {ENDPOINT_LABEL} skipped: empty rom_name");
			return Ok(None);
		}

		let md5 = md5.map(str::trim).filter(|s| !s.is_empty());
		let sha1 = sha1.map(str::trim).filter(|s| !s.is_empty());
		let crc = crc.map(str::trim).filter(|s| !s.is_empty());
		if md5.is_none() && sha1.is_none() && crc.is_none() {
			debug!("screenscraper {ENDPOINT_LABEL} skipped: no hashes available");
			return Ok(None);
		}

		let mut params: Vec<(&'static str, String)> = vec![
			("systemeid", system_id.to_string()),
			("romtype", "rom".to_string()),
			("romnom", trimmed_name.to_string()),
		];
		if let Some(v) = sha1 {
			params.push(("sha1", v.to_string()));
		}
		if let Some(v) = md5 {
			params.push(("md5", v.to_string()));
		}
		if let Some(v) = crc {
			params.push(("crc", v.to_string()));
		}
		if let Some(size) = rom_size.filter(|s| *s > 0) {
			params.push(("romtaille", size.to_string()));
		}

		let url = self.url("jeuInfos.php", &params)?;
		self.fetch_optional_game(ENDPOINT_LABEL, url).await
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

	fn dev_query_pairs(&self) -> Vec<(&'static str, String)> {
		vec![
			("devid", self.dev_id.clone()),
			("devpassword", self.dev_password.clone()),
			("softname", SOFTNAME.to_string()),
			("output", "json".to_string()),
		]
	}

	fn url(&self, endpoint: &str, extra: &[(&'static str, String)]) -> anyhow::Result<Url> {
		let mut url = Url::parse(API_URL)?;
		url.path_segments_mut()
			.map_err(|_| anyhow!("screenscraper base url cannot have path segments"))?
			.push(endpoint);
		let mut pairs = self.dev_query_pairs();
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
	/// quota and concurrency state from the envelope's `ssuser` block on
	/// success.
	async fn do_get_envelope_optional<T: DeserializeOwned>(
		&self,
		endpoint_label: &'static str,
		url: Url,
	) -> anyhow::Result<Option<SsEnvelope<T>>> {
		let started = std::time::Instant::now();
		let sanitised_url = url_for_log(&url);
		let result = self.execute_get(url).await;
		let outcome = match &result {
			Ok((status, _, _, _)) if status.is_success() => "success",
			Ok((status, _, _, _)) if *status == StatusCode::NOT_FOUND => "not_found",
			Ok(_) => "error",
			Err(_) => "error",
		};
		crate::metrics::record_metadata_request(
			"screenscraper",
			endpoint_label,
			outcome,
			started.elapsed().as_secs_f64(),
		);

		let (status, content_type, body, account) = result?;

		match status {
			s if s == StatusCode::NOT_FOUND => Ok(None),
			s if !s.is_success() => Err(anyhow!(
				"screenscraper {endpoint_label} returned non-success status: {s} for {sanitised_url} (body preview: {:?})",
				body_preview(&body)
			)),
			_ if body.is_empty() => Ok(None),
			_ => match parse_or_incident(&body, content_type.as_deref())? {
				ParseOutcome::NotFound => Ok(None),
				ParseOutcome::Parseable => {
					let env: SsEnvelope<T> = serde_json::from_str(&body)
						.with_context(|| "failed to parse screenscraper response envelope")?;
					if let Some(resp) = env.response.as_ref()
						&& let Some(account_ref) = account.as_ref()
					{
						self.update_quota_from(account_ref, &resp.ssuser);
						self.update_concurrency_from(account_ref, &resp.ssuser);
					}
					let header_signals_failure = env
						.header
						.as_ref()
						.is_some_and(|h| h.success.eq_ignore_ascii_case("false"));
					if header_signals_failure {
						debug!(
							"screenscraper header.success=false for {endpoint_label}: {:?}",
							env.header.and_then(|h| h.error)
						);
						return Ok(None);
					}
					Ok(Some(env))
				}
			},
		}
	}

	async fn execute_get(
		&self,
		url: Url,
	) -> anyhow::Result<(StatusCode, Option<String>, String, Option<Arc<Account>>)> {
		if self.blacklisted.load(Ordering::Relaxed) {
			return Err(anyhow!(
				"screenscraper client is blacklisted (HTTP 426 received earlier); restart required after fixing the integration"
			));
		}
		let now = now_unix_secs();
		let outage_until = self.outage_until_unix.load(Ordering::Relaxed);
		if outage_until > now {
			return Err(anyhow!(
				"screenscraper api in outage window (HTTP 423); retry in {}s",
				outage_until - now
			));
		}

		let account = self.current_account().await?;

		let mut url_for_call = url.clone();
		if let Some(a) = account.as_ref() {
			url_for_call
				.query_pairs_mut()
				.append_pair("ssid", &a.id)
				.append_pair("sspassword", &a.password);
		}

		let _permit = match account.as_ref() {
			Some(a) => Some(a.permits.clone().acquire_owned().await?),
			None => None,
		};

		let mut attempt: usize = 0;
		let outcome = loop {
			let (status, content_type, body) = self.send_one(url_for_call.clone()).await?;
			match status.as_u16() {
				429 if attempt < MAX_429_RETRIES => {
					let base = RETRY_BACKOFF_MS[attempt.min(RETRY_BACKOFF_MS.len() - 1)];
					let jitter: u64 = rand::rng().random_range(0..=100);
					sleep(Duration::from_millis(base + jitter)).await;
					attempt += 1;
					continue;
				}
				429 => break Outcome::Backoff429,
				423 => break Outcome::Outage,
				426 => break Outcome::Blacklist,
				430 | 431 => break Outcome::AccountExhausted(status.as_u16()),
				_ => break Outcome::Done(status, content_type, body),
			}
		};

		sleep(Duration::from_millis(POST_REQUEST_DELAY_MS)).await;
		drop(_permit);

		match outcome {
			Outcome::Done(status, ct, body) => Ok((status, ct, body, account)),
			Outcome::Backoff429 => Err(anyhow!(
				"screenscraper 429 thread-limit; exhausted {MAX_429_RETRIES} retries"
			)),
			Outcome::Outage => {
				self.outage_until_unix
					.store(now_unix_secs() + OUTAGE_BLOCK_SECS, Ordering::Relaxed);
				crate::metrics::record_screenscraper_quota_exhaustion("http_423");
				Err(anyhow!(
					"screenscraper api totally closed (HTTP 423), outage block engaged"
				))
			}
			Outcome::Blacklist => {
				self.blacklisted.store(true, Ordering::Relaxed);
				crate::metrics::record_screenscraper_quota_exhaustion("http_426");
				error!(
					"screenscraper client blacklisted (HTTP 426); the integration is non-compliant or obsolete and requires a code update"
				);
				Err(anyhow!(
					"screenscraper client blacklisted (HTTP 426), requires update"
				))
			}
			Outcome::AccountExhausted(code) => {
				if let Some(a) = account.as_ref() {
					self.mark_account_exhausted(a, code).await;
				}
				Err(anyhow!(
					"screenscraper account exhausted (HTTP {code}); resuming at next paris midnight"
				))
			}
		}
	}

	async fn send_one(&self, url: Url) -> anyhow::Result<(StatusCode, Option<String>, String)> {
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

		let inflight = self.service.lock().await.ready().await?.call(req);
		let res = inflight.await?;
		let status = res.status();
		let content_type = res
			.headers()
			.get(reqwest::header::CONTENT_TYPE)
			.and_then(|v| v.to_str().ok())
			.map(|s| s.to_string());
		let body = res.text().await?;
		Ok((status, content_type, body))
	}

	async fn current_account(&self) -> anyhow::Result<Option<Arc<Account>>> {
		let Some(account) = self.account.as_ref() else {
			return Ok(None);
		};
		self.refresh_account_state_from_redis().await;
		let now = now_unix_secs();
		if account.exhausted_until_unix.load(Ordering::Relaxed) > now {
			return Err(anyhow!(
				"screenscraper account exhausted; waiting for daily reset"
			));
		}
		account.last_used_at_unix.store(now, Ordering::Relaxed);
		Ok(Some(account.clone()))
	}

	async fn refresh_account_state_from_redis(&self) {
		let Some(account) = self.account.as_ref() else {
			return;
		};
		let mut conn = self.redis_conn.clone();
		let now = now_unix_secs();
		for a in std::iter::once(account) {
			if a.exhausted_until_unix.load(Ordering::Relaxed) <= now {
				let val: Result<Option<String>, _> = conn.get(&a.redis_key).await;
				if let Ok(Some(raw)) = val
					&& let Ok(ts) = raw.parse::<i64>()
					&& ts > now
				{
					a.exhausted_until_unix.store(ts, Ordering::Relaxed);
				}
			}

			if a.exhausted_until_unix.load(Ordering::Relaxed) > now {
				continue;
			}

			let snap_raw: Result<Option<String>, _> = conn.get(&a.quota_redis_key).await;
			if let Ok(Some(raw)) = snap_raw
				&& let Ok(snapshot) = serde_json::from_str::<QuotaSnapshot>(&raw)
			{
				let ok_hit = soft_limit_reached(
					Some(snapshot.requests_today),
					Some(snapshot.max_requests_per_day),
				);
				let ko_hit = soft_limit_reached(
					Some(snapshot.requests_ko_today),
					Some(snapshot.max_requests_ko_per_day),
				);
				if ok_hit || ko_hit {
					let reset = next_paris_midnight_unix(now);
					a.exhausted_until_unix.store(reset, Ordering::Relaxed);
					crate::metrics::record_screenscraper_quota_exhaustion(if ok_hit {
						"ok_soft_limit_restored"
					} else {
						"ko_soft_limit_restored"
					});
					let reason = if ok_hit {
						format!(
							"OK quota soft limit (restored from snapshot, {}/{})",
							snapshot.requests_today, snapshot.max_requests_per_day
						)
					} else {
						format!(
							"KO quota soft limit (restored from snapshot, {}/{})",
							snapshot.requests_ko_today, snapshot.max_requests_ko_per_day
						)
					};
					info!(
						"screenscraper account exhausted: {reason}; resuming at next paris midnight (~{}s)",
						(reset - now).max(0)
					);
					let ttl = (reset - now).max(60) as u64;
					let mut c = conn.clone();
					let key = a.redis_key.clone();
					let value = reset.to_string();
					tokio::spawn(async move {
						let _: Result<(), _> = c.set_ex(&key, value, ttl).await;
					});
				}
			}
		}
	}

	async fn mark_account_exhausted(&self, account: &Account, http_code: u16) {
		let now = now_unix_secs();
		let reset = next_paris_midnight_unix(now);
		account.exhausted_until_unix.store(reset, Ordering::Relaxed);
		crate::metrics::record_screenscraper_quota_exhaustion(&format!("http_{http_code}"));
		let reason = match http_code {
			430 => "OK quota hard limit (HTTP 430)",
			431 => "KO quota hard limit (HTTP 431)",
			_ => "hard quota limit",
		};
		info!(
			"screenscraper account exhausted: {reason}; resuming at next paris midnight (~{}s)",
			(reset - now).max(0)
		);
		let ttl = (reset - now).max(60) as u64;
		let mut conn = self.redis_conn.clone();
		let key = account.redis_key.clone();
		let value = reset.to_string();
		tokio::spawn(async move {
			let _: Result<(), _> = conn.set_ex(&key, value, ttl).await;
		});
	}

	fn update_quota_from(&self, account: &Account, user: &Option<SsUser>) {
		let Some(user) = user else { return };
		let snapshot = QuotaSnapshot {
			requests_today: parsed_u64(user.requeststoday.as_deref()).unwrap_or(0),
			max_requests_per_day: parsed_u64(user.maxrequestsperday.as_deref()).unwrap_or(0),
			requests_ko_today: parsed_u64(user.requestskotoday.as_deref()).unwrap_or(0),
			max_requests_ko_per_day: parsed_u64(user.maxrequestskoperday.as_deref()).unwrap_or(0),
		};
		self.persist_quota_snapshot(account, &snapshot);

		let ok_hit = soft_limit_reached(
			Some(snapshot.requests_today),
			Some(snapshot.max_requests_per_day),
		);
		let ko_hit = soft_limit_reached(
			Some(snapshot.requests_ko_today),
			Some(snapshot.max_requests_ko_per_day),
		);
		if !ok_hit && !ko_hit {
			return;
		}
		let now = now_unix_secs();
		if account.exhausted_until_unix.load(Ordering::Relaxed) > now {
			return;
		}
		let reset = next_paris_midnight_unix(now);
		account.exhausted_until_unix.store(reset, Ordering::Relaxed);
		crate::metrics::record_screenscraper_quota_exhaustion(if ok_hit {
			"ok_soft_limit"
		} else {
			"ko_soft_limit"
		});
		let reason = if ok_hit {
			format!(
				"OK quota soft limit ({}/{})",
				snapshot.requests_today, snapshot.max_requests_per_day
			)
		} else {
			format!(
				"KO quota soft limit ({}/{})",
				snapshot.requests_ko_today, snapshot.max_requests_ko_per_day
			)
		};
		info!(
			"screenscraper account exhausted: {reason}; resuming at next paris midnight (~{}s)",
			(reset - now).max(0)
		);
		let ttl = (reset - now).max(60) as u64;
		let mut conn = self.redis_conn.clone();
		let key = account.redis_key.clone();
		let value = reset.to_string();
		tokio::spawn(async move {
			let _: Result<(), _> = conn.set_ex(&key, value, ttl).await;
		});
	}

	fn persist_quota_snapshot(&self, account: &Account, snapshot: &QuotaSnapshot) {
		let Ok(value) = serde_json::to_string(snapshot) else {
			return;
		};
		let now = now_unix_secs();
		let reset = next_paris_midnight_unix(now);
		let ttl = (reset - now).max(60) as u64;
		let mut conn = self.redis_conn.clone();
		let key = account.quota_redis_key.clone();
		tokio::spawn(async move {
			let _: Result<(), _> = conn.set_ex(&key, value, ttl).await;
		});
	}

	fn update_concurrency_from(&self, account: &Account, user: &Option<SsUser>) {
		let Some(user) = user else { return };
		let Some(target) = parse_maxthreads(user) else {
			return;
		};
		let current = account.concurrency.load(Ordering::Relaxed);
		if target > current {
			account.permits.add_permits(target - current);
			account.concurrency.store(target, Ordering::Relaxed);
			crate::metrics::set_screenscraper_concurrency(target as i64);
			info!(
				"screenscraper concurrency raised to {target} from ssuser.maxthreads (was {current})"
			);
		}
	}
}

enum Outcome {
	Done(StatusCode, Option<String>, String),
	Backoff429,
	Outage,
	Blacklist,
	AccountExhausted(u16),
}

fn parse_maxthreads(user: &SsUser) -> Option<usize> {
	let raw = user.maxthreads.as_deref()?;
	let parsed = raw.parse::<usize>().ok()?;
	if parsed == 0 {
		return None;
	}
	Some(parsed.min(MAX_CONCURRENCY))
}

fn parsed_u64(value: Option<&str>) -> Option<u64> {
	value?.parse::<u64>().ok()
}

fn soft_limit_reached(today: Option<u64>, max: Option<u64>) -> bool {
	let (Some(today), Some(max)) = (today, max) else {
		return false;
	};
	if max == 0 {
		return false;
	}
	today * QUOTA_SOFT_LIMIT_DENOMINATOR >= max * QUOTA_SOFT_LIMIT_NUMERATOR
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

fn compute_secs_until_recovery(
	blacklisted: bool,
	outage_until_unix: i64,
	pool_exhaustion_unix: &[i64],
	now: i64,
) -> Option<u64> {
	if blacklisted {
		return None;
	}
	let pool_earliest = pool_exhaustion_unix
		.iter()
		.copied()
		.filter(|t| *t > now)
		.min()
		.unwrap_or(0);
	let target = outage_until_unix.max(pool_earliest);
	if target > now {
		Some((target - now) as u64)
	} else {
		None
	}
}

fn now_unix_secs() -> i64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(|d| d.as_secs() as i64)
		.unwrap_or(0)
}

fn next_paris_midnight_unix(now_unix: i64) -> i64 {
	let now_utc = Utc
		.timestamp_opt(now_unix, 0)
		.single()
		.unwrap_or_else(Utc::now);
	let now_paris = now_utc.with_timezone(&Paris);
	let tomorrow = (now_paris + ChronoDuration::days(1)).date_naive();
	if let Some(midnight) = tomorrow.and_hms_opt(0, 0, 0) {
		match Paris.from_local_datetime(&midnight) {
			chrono::LocalResult::Single(dt) => return dt.timestamp(),
			chrono::LocalResult::Ambiguous(_, dt) => return dt.timestamp(),
			chrono::LocalResult::None => {
				if let Some(fallback) = tomorrow.and_hms_opt(1, 0, 0)
					&& let chrono::LocalResult::Single(dt) = Paris.from_local_datetime(&fallback)
				{
					return dt.timestamp();
				}
			}
		}
	}
	now_unix + 86400
}

fn sha256_hex(value: &str) -> String {
	hex::encode(Sha256::digest(value.as_bytes()))
}

/// ScreenScraper's PHP backend reports `systemeid=0` as missing because
/// PHP's `empty("0")` is true. Treat any non-positive value the same way
/// so we never spend a request on a guaranteed 400.
fn valid_system_id(system_id: i32) -> bool {
	system_id > 0
}

/// Trims the body for inclusion in error messages on non-success statuses.
/// Error responses do not contain the `ssuser` block (that only rides on
/// success envelopes), so they are safe to log unredacted within a short cap.
fn body_preview(body: &str) -> String {
	const LIMIT: usize = 256;
	let collapsed: String = body.split_whitespace().collect::<Vec<_>>().join(" ");
	collapsed.chars().take(LIMIT).collect()
}

/// Returns `Err` when the body looks like a French incident message or the
/// content type is not JSON, so the caller does not try to parse the body and
/// the per-game match logs an error instead of aborting the whole cycle.
enum ParseOutcome {
	Parseable,
	NotFound,
}

fn parse_or_incident(body: &str, content_type: Option<&str>) -> anyhow::Result<ParseOutcome> {
	if let Some(ct) = content_type
		&& !ct.to_ascii_lowercase().contains("application/json")
	{
		return Err(anyhow!(
			"screenscraper returned non-json content-type {ct}, treating as incident"
		));
	}

	let prefix: String = body.chars().take(1024).collect();
	let prefix_lower = prefix.to_lowercase();
	for phrase in NOT_FOUND_PHRASES {
		if prefix_lower.contains(&phrase.to_lowercase()) {
			return Ok(ParseOutcome::NotFound);
		}
	}
	for phrase in INCIDENT_PHRASES {
		if prefix_lower.contains(&phrase.to_lowercase()) {
			return Err(anyhow!(
				"screenscraper response matched incident phrase: {phrase}"
			));
		}
	}

	Ok(ParseOutcome::Parseable)
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
		let now = now_unix_secs();
		self.account
			.as_ref()
			.filter(|a| a.exhausted_until_unix.load(Ordering::Relaxed) <= now)
			.map(|a| a.concurrency.load(Ordering::Relaxed))
			.unwrap_or(1)
			.clamp(1, MAX_CONCURRENCY)
	}

	fn redis_conn(&self) -> &redis::aio::MultiplexedConnection {
		&self.redis_conn
	}

	async fn match_db(self: Arc<Self>, db_conn: &sea_orm::DbConn) -> anyhow::Result<()> {
		if self.is_quota_exhausted() {
			warn!("screenscraper quota exhausted at cycle start, skipping");
			return Ok(());
		}
		matching::match_db_to_screenscraper_entities(self, db_conn).await
	}

	async fn match_via_sibling_names(
		self: Arc<Self>,
		db_conn: &sea_orm::DbConn,
	) -> anyhow::Result<()> {
		if self.is_quota_exhausted() {
			warn!("screenscraper quota exhausted at cross-pass start, skipping");
			return Ok(());
		}
		crate::providers::drive_cross_match_pipeline(
			"screenscraper",
			MetadataProviderEnum::Screenscraper,
			matching::game::match_game_via_sibling_name_screenscraper,
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
	fn valid_system_id_rejects_zero_and_negative() {
		assert!(!valid_system_id(0));
		assert!(!valid_system_id(-1));
		assert!(valid_system_id(1));
		assert!(valid_system_id(57));
	}

	#[test]
	fn body_preview_collapses_whitespace_and_caps_length() {
		let body = "Recherche\n\ntrop\tcourte";
		assert_eq!(body_preview(body), "Recherche trop courte");
		let long: String = "a".repeat(1024);
		assert_eq!(body_preview(&long).chars().count(), 256);
	}

	#[test]
	fn parse_or_incident_accepts_valid_json_body() {
		let body = "{\"header\": {\"success\": \"true\"}}";
		assert!(matches!(
			parse_or_incident(body, Some("application/json")),
			Ok(ParseOutcome::Parseable)
		));
	}

	#[test]
	fn parse_or_incident_classifies_not_found_phrases_as_misses() {
		for phrase in NOT_FOUND_PHRASES {
			let body = format!("Erreur : ROM/ISO/Fichier {phrase} !");
			assert!(
				matches!(
					parse_or_incident(&body, Some("application/json")),
					Ok(ParseOutcome::NotFound)
				),
				"phrase {phrase} should classify as NotFound, got something else"
			);
		}
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

	fn user_with(
		today: Option<&str>,
		max: Option<&str>,
		ko: Option<&str>,
		kmax: Option<&str>,
	) -> SsUser {
		SsUser {
			requeststoday: today.map(str::to_string),
			maxrequestsperday: max.map(str::to_string),
			requestskotoday: ko.map(str::to_string),
			maxrequestskoperday: kmax.map(str::to_string),
			maxrequestspermin: None,
			maxthreads: None,
		}
	}

	#[test]
	fn soft_limit_triggers_on_ok_quota_above_threshold() {
		let u = user_with(Some("950"), Some("1000"), None, None);
		assert!(soft_limit_reached(
			parsed_u64(u.requeststoday.as_deref()),
			parsed_u64(u.maxrequestsperday.as_deref())
		));
	}

	#[test]
	fn soft_limit_does_not_trigger_below_threshold() {
		let u = user_with(Some("900"), Some("1000"), None, None);
		assert!(!soft_limit_reached(
			parsed_u64(u.requeststoday.as_deref()),
			parsed_u64(u.maxrequestsperday.as_deref())
		));
	}

	#[test]
	fn soft_limit_safe_when_max_unknown_or_zero() {
		let u = user_with(Some("950"), None, None, None);
		assert!(!soft_limit_reached(
			parsed_u64(u.requeststoday.as_deref()),
			parsed_u64(u.maxrequestsperday.as_deref())
		));
		let u = user_with(Some("0"), Some("0"), None, None);
		assert!(!soft_limit_reached(
			parsed_u64(u.requeststoday.as_deref()),
			parsed_u64(u.maxrequestsperday.as_deref())
		));
	}

	#[test]
	fn soft_limit_triggers_on_ko_quota_independently() {
		let u = user_with(Some("0"), Some("1000"), Some("96"), Some("100"));
		assert!(!soft_limit_reached(
			parsed_u64(u.requeststoday.as_deref()),
			parsed_u64(u.maxrequestsperday.as_deref())
		));
		assert!(soft_limit_reached(
			parsed_u64(u.requestskotoday.as_deref()),
			parsed_u64(u.maxrequestskoperday.as_deref())
		));
	}

	fn user_with_maxthreads(value: Option<&str>) -> SsUser {
		SsUser {
			requeststoday: None,
			maxrequestsperday: None,
			requestskotoday: None,
			maxrequestskoperday: None,
			maxrequestspermin: None,
			maxthreads: value.map(str::to_string),
		}
	}

	#[test]
	fn parse_maxthreads_accepts_typical_tier_values() {
		assert_eq!(parse_maxthreads(&user_with_maxthreads(Some("1"))), Some(1));
		assert_eq!(parse_maxthreads(&user_with_maxthreads(Some("5"))), Some(5));
		assert_eq!(
			parse_maxthreads(&user_with_maxthreads(Some("16"))),
			Some(16)
		);
	}

	#[test]
	fn parse_maxthreads_clamps_to_max_concurrency() {
		assert_eq!(
			parse_maxthreads(&user_with_maxthreads(Some("100"))),
			Some(MAX_CONCURRENCY)
		);
		assert_eq!(
			parse_maxthreads(&user_with_maxthreads(Some("17"))),
			Some(MAX_CONCURRENCY)
		);
	}

	#[test]
	fn parse_maxthreads_rejects_garbage_and_missing() {
		assert_eq!(parse_maxthreads(&user_with_maxthreads(None)), None);
		assert_eq!(parse_maxthreads(&user_with_maxthreads(Some(""))), None);
		assert_eq!(parse_maxthreads(&user_with_maxthreads(Some("abc"))), None);
		assert_eq!(parse_maxthreads(&user_with_maxthreads(Some("0"))), None);
		assert_eq!(parse_maxthreads(&user_with_maxthreads(Some("-1"))), None);
	}

	#[test]
	fn next_paris_midnight_is_future_and_within_two_days() {
		let now = now_unix_secs();
		let reset = next_paris_midnight_unix(now);
		assert!(reset > now);
		assert!(reset - now < 2 * 86400);
	}

	#[test]
	fn sha256_hex_is_64_chars_lowercase() {
		let hex = sha256_hex("hello");
		assert_eq!(hex.len(), 64);
		assert!(
			hex.chars()
				.all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
		);
	}

	#[test]
	fn secs_until_recovery_returns_none_when_nothing_exhausted() {
		let now = 1_000_000;
		assert_eq!(compute_secs_until_recovery(false, 0, &[0, 0], now), None);
	}

	#[test]
	fn secs_until_recovery_returns_none_when_blacklisted() {
		let now = 1_000_000;
		assert_eq!(
			compute_secs_until_recovery(true, now + 600, &[now + 3600], now),
			None
		);
	}

	#[test]
	fn secs_until_recovery_uses_outage_when_pool_clean() {
		let now = 1_000_000;
		assert_eq!(
			compute_secs_until_recovery(false, now + 120, &[0], now),
			Some(120)
		);
	}

	#[test]
	fn secs_until_recovery_uses_pool_earliest_when_outage_clean() {
		let now = 1_000_000;
		assert_eq!(
			compute_secs_until_recovery(false, 0, &[now + 600, now + 300], now),
			Some(300)
		);
	}

	#[test]
	fn secs_until_recovery_takes_max_of_outage_and_pool() {
		let now = 1_000_000;
		assert_eq!(
			compute_secs_until_recovery(false, now + 600, &[now + 120], now),
			Some(600)
		);
	}

	#[test]
	fn account_has_distinct_redis_keys_for_exhaustion_and_quota() {
		let acc = Account::new("user42".into(), "pw".into());
		assert!(acc.redis_key.ends_with(":exhausted_until"));
		assert!(acc.quota_redis_key.ends_with(":quota"));
		assert_ne!(acc.redis_key, acc.quota_redis_key);
		assert!(acc.redis_key.starts_with(REDIS_KEY_PREFIX));
		assert!(acc.quota_redis_key.starts_with(REDIS_KEY_PREFIX));
	}

	#[test]
	fn quota_snapshot_round_trips_through_json() {
		let snap = QuotaSnapshot {
			requests_today: 19_000,
			max_requests_per_day: 20_000,
			requests_ko_today: 630,
			max_requests_ko_per_day: 2_000,
		};
		let raw = serde_json::to_string(&snap).expect("serialize");
		let back: QuotaSnapshot = serde_json::from_str(&raw).expect("deserialize");
		assert_eq!(snap, back);
	}

	#[test]
	fn quota_snapshot_soft_limit_re_check_matches_runtime_check() {
		fn check(today: u64, max: u64) -> bool {
			let snap = QuotaSnapshot {
				requests_today: today,
				max_requests_per_day: max,
				requests_ko_today: 0,
				max_requests_ko_per_day: 0,
			};
			let from_snapshot =
				soft_limit_reached(Some(snap.requests_today), Some(snap.max_requests_per_day));
			let u = user_with(Some(&today.to_string()), Some(&max.to_string()), None, None);
			let from_runtime = soft_limit_reached(
				parsed_u64(u.requeststoday.as_deref()),
				parsed_u64(u.maxrequestsperday.as_deref()),
			);
			assert_eq!(from_snapshot, from_runtime);
			from_snapshot
		}
		assert!(!check(9_400, 10_000));
		assert!(check(9_500, 10_000));
		assert!(check(9_600, 10_000));
	}
}
