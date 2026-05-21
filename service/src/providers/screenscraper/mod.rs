use crate::config::http::REQWEST_DEFAULT_USER_AGENT;
use crate::http::abstraction::RetryPolicy;
use crate::providers::screenscraper::model::{
	JeuPayload, JeuxPayload, SsEnvelope, SsGame, SsSystem, SsUser, SystemesPayload,
};
use anyhow::{Context, anyhow};
use entity::sea_orm_active_enums::MetadataProviderEnum;
use log::{debug, info, warn};
use reqwest::header::HeaderMap;
use reqwest::{Client, Method, StatusCode, Url};
use serde::de::DeserializeOwned;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
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
const MAX_RETRIES: usize = 3;

/// Hard ceiling on the concurrency probed from `ssuser.maxthreads`.
/// Defends against pathological response payloads returning a huge
/// number that would otherwise spawn that many tokio tasks per
/// match-cycle page.
const MAX_CONCURRENCY: usize = 16;

/// At or above this fraction of the daily request budget we stop the cycle
/// early so the cron resumes after the daily reset rather than burning the
/// last few requests on partial work.
const QUOTA_SOFT_LIMIT_NUMERATOR: u64 = 95;
const QUOTA_SOFT_LIMIT_DENOMINATOR: u64 = 100;

/// How long an exhaustion mark blocks further requests. ScreenScraper's
/// daily request budget resets every 24 hours, so any block (quota,
/// thread limit, transient overload) is safe to retry past this window.
const QUOTA_BLOCK_TTL_SECS: i64 = 24 * 60 * 60;

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

pub struct ScreenScraperClient {
	client: Client,
	service: Mutex<Retry<RetryPolicy, Client>>,
	dev_id: String,
	dev_password: String,
	user: Option<(String, String)>,
	quota_exhausted_at: AtomicI64,
	systems_cache: OnceCell<Arc<Vec<SsSystem>>>,
	redis_conn: redis::aio::MultiplexedConnection,
	concurrency: AtomicUsize,
	permits: Arc<Semaphore>,
}

impl ScreenScraperClient {
	pub fn new(
		dev_id: String,
		dev_password: String,
		user: Option<(String, String)>,
		client: Client,
		redis_conn: redis::aio::MultiplexedConnection,
	) -> anyhow::Result<Self> {
		let retry_layer = tower::retry::RetryLayer::new(RetryPolicy(MAX_RETRIES));

		let service = ServiceBuilder::new()
			.layer(retry_layer)
			.service(client.clone());

		crate::metrics::set_screenscraper_concurrency(1);
		Ok(Self {
			client,
			service: Mutex::new(service),
			dev_id,
			dev_password,
			user,
			quota_exhausted_at: AtomicI64::new(0),
			systems_cache: OnceCell::new(),
			redis_conn,
			concurrency: AtomicUsize::new(1),
			permits: Arc::new(Semaphore::new(1)),
		})
	}

	pub fn is_quota_exhausted(&self) -> bool {
		let stamp = self.quota_exhausted_at.load(Ordering::Relaxed);
		if !is_within_quota_block(stamp, now_unix_secs()) {
			if stamp != 0 {
				let _ = self.quota_exhausted_at.compare_exchange(
					stamp,
					0,
					Ordering::Relaxed,
					Ordering::Relaxed,
				);
			}
			return false;
		}
		true
	}

	/// Records a fresh exhaustion timestamp and returns whether this is a
	/// new transition (the previous mark was unset or already past its
	/// 24h TTL). Callers that emit metrics or warn-level logs gate on the
	/// returned bool so we don't spam on every repeat 430.
	fn mark_quota_exhausted(&self, reason: &str) -> bool {
		let now = now_unix_secs();
		let prev = self.quota_exhausted_at.swap(now, Ordering::Relaxed);
		let was_fresh = !is_within_quota_block(prev, now);
		if was_fresh {
			crate::metrics::record_screenscraper_quota_exhaustion(reason);
		}
		was_fresh
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
		let sanitised_url = url_for_log(&url);
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
				self.mark_quota_exhausted("http_430");
				Err(anyhow!(
					"screenscraper daily quota exhausted (HTTP 430), aborting cycle"
				))
			}
			s if matches!(s.as_u16(), 401 | 426 | 429 | 431) => {
				self.mark_quota_exhausted(&format!("http_{}", s.as_u16()));
				Err(anyhow!(
					"screenscraper rejected the request with HTTP {s} (server overloaded or thread limit), aborting cycle"
				))
			}
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
					if let Some(resp) = env.response.as_ref() {
						self.update_quota_from(&resp.ssuser);
						self.update_concurrency_from(&resp.ssuser);
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

		// Owned permit so we can hold it across the request future and the
		// post-request sleep without borrowing self. The mutex around the
		// retry stack only serialises the brief poll-readiness call; the
		// HTTP work itself runs unlocked, gated by the semaphore.
		let _permit = self.permits.clone().acquire_owned().await?;

		let inflight = self.service.lock().await.ready().await?.call(req);
		let res = inflight.await?;
		let status = res.status();
		let content_type = res
			.headers()
			.get(reqwest::header::CONTENT_TYPE)
			.and_then(|v| v.to_str().ok())
			.map(|s| s.to_string());
		let body = res.text().await?;

		// Body preview deliberately omitted: the response embeds an `ssuser`
		// block with the account username, numeric user id, tier, and last
		// visit, which would leak through the debug logs.

		// Hold the permit through the courtesy interval so each thread
		// paces itself before releasing the slot for the next caller.
		sleep(Duration::from_millis(POST_REQUEST_DELAY_MS)).await;

		Ok((status, content_type, body))
	}

	fn update_quota_from(&self, user: &Option<SsUser>) {
		let Some(user) = user else { return };
		if quota_should_mark_exhausted(user) && self.mark_quota_exhausted("ssuser_threshold") {
			let (today, max) = parsed_quota(user).unwrap_or((0, 0));
			warn!(
				"screenscraper quota near limit ({today}/{max}); short-circuiting remaining match cycle"
			);
		}
	}

	/// Raise the semaphore and concurrency counter when `ssuser.maxthreads`
	/// exceeds the current value. Monotonic; clamped by `MAX_CONCURRENCY`.
	fn update_concurrency_from(&self, user: &Option<SsUser>) {
		let Some(user) = user else { return };
		let Some(target) = parse_maxthreads(user) else {
			return;
		};
		let current = self.concurrency.load(Ordering::Relaxed);
		if target > current {
			self.permits.add_permits(target - current);
			self.concurrency.store(target, Ordering::Relaxed);
			crate::metrics::set_screenscraper_concurrency(target as i64);
			info!(
				"screenscraper concurrency raised to {target} from ssuser.maxthreads (was {current})"
			);
		}
	}
}

/// Returns the clamped concurrency target, or `None` if the field is
/// missing or unparseable. We default to 1 in those cases by leaving the
/// existing value untouched.
fn parse_maxthreads(user: &SsUser) -> Option<usize> {
	let raw = user.maxthreads.as_deref()?;
	let parsed = raw.parse::<usize>().ok()?;
	if parsed == 0 {
		return None;
	}
	Some(parsed.min(MAX_CONCURRENCY))
}

fn parsed_quota(user: &SsUser) -> Option<(u64, u64)> {
	let today = user
		.requeststoday
		.as_deref()
		.and_then(|s| s.parse::<u64>().ok())?;
	let max = user
		.maxrequestsperday
		.as_deref()
		.and_then(|s| s.parse::<u64>().ok())?;
	if max == 0 {
		return None;
	}
	Some((today, max))
}

fn quota_should_mark_exhausted(user: &SsUser) -> bool {
	match parsed_quota(user) {
		Some((today, max)) => {
			today * QUOTA_SOFT_LIMIT_DENOMINATOR >= max * QUOTA_SOFT_LIMIT_NUMERATOR
		}
		None => false,
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

fn now_unix_secs() -> i64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(|d| d.as_secs() as i64)
		.unwrap_or(0)
}

/// True while a non-zero `stamp` is younger than [`QUOTA_BLOCK_TTL_SECS`].
/// Older stamps are treated as cleared so the next match cycle retries.
fn is_within_quota_block(stamp: i64, now: i64) -> bool {
	stamp != 0 && now.saturating_sub(stamp) < QUOTA_BLOCK_TTL_SECS
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
		self.concurrency
			.load(Ordering::Relaxed)
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
	fn quota_block_is_active_during_first_24h_and_clears_after() {
		let set_at = 1_000_000;
		assert!(is_within_quota_block(set_at, set_at));
		assert!(is_within_quota_block(set_at, set_at + 1));
		assert!(is_within_quota_block(
			set_at,
			set_at + QUOTA_BLOCK_TTL_SECS - 1
		));
		assert!(!is_within_quota_block(
			set_at,
			set_at + QUOTA_BLOCK_TTL_SECS
		));
		assert!(!is_within_quota_block(
			set_at,
			set_at + QUOTA_BLOCK_TTL_SECS + 60
		));
	}

	#[test]
	fn quota_block_unset_stamp_is_never_active() {
		assert!(!is_within_quota_block(0, 0));
		assert!(!is_within_quota_block(0, 1_000_000));
	}

	#[test]
	fn quota_block_tolerates_clock_skew() {
		let set_at = 2_000_000;
		assert!(is_within_quota_block(set_at, set_at - 5));
		assert!(is_within_quota_block(set_at, 0));
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

	#[test]
	fn quota_marks_exhausted_at_or_above_95_percent() {
		let user = SsUser {
			requeststoday: Some("950".into()),
			maxrequestsperday: Some("1000".into()),
			maxrequestspermin: None,
			maxthreads: None,
		};
		assert!(quota_should_mark_exhausted(&user));
	}

	#[test]
	fn quota_does_not_mark_exhausted_below_95_percent() {
		let user = SsUser {
			requeststoday: Some("900".into()),
			maxrequestsperday: Some("1000".into()),
			maxrequestspermin: None,
			maxthreads: None,
		};
		assert!(!quota_should_mark_exhausted(&user));
	}

	#[test]
	fn quota_does_not_mark_exhausted_with_unparseable_max() {
		let user = SsUser {
			requeststoday: Some("950".into()),
			maxrequestsperday: None,
			maxrequestspermin: None,
			maxthreads: None,
		};
		assert!(!quota_should_mark_exhausted(&user));
	}

	#[test]
	fn quota_does_not_mark_exhausted_with_zero_max() {
		let user = SsUser {
			requeststoday: Some("0".into()),
			maxrequestsperday: Some("0".into()),
			maxrequestspermin: None,
			maxthreads: None,
		};
		assert!(!quota_should_mark_exhausted(&user));
	}

	fn user_with_maxthreads(value: Option<&str>) -> SsUser {
		SsUser {
			requeststoday: None,
			maxrequestsperday: None,
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
}
