use crate::config::http::REQWEST_DEFAULT_USER_AGENT;
use crate::providers::thegamesdb::TheGamesDbClient;
use crate::providers::thegamesdb::model::{
	TgdbApiGame, TgdbEnvelope, TgdbGameByIdData, TgdbSearchData,
};
use anyhow::{Context, anyhow};
use log::debug;
use redis::AsyncTypedCommands;
use reqwest::header::HeaderMap;
use reqwest::{Method, StatusCode, Url};
use serde::de::DeserializeOwned;
use std::sync::atomic::Ordering;
use std::time::Instant;

pub const API_BASE: &str = "https://api.thegamesdb.net";

/// Returned by [`TheGamesDbClient::search_by_name`] when the per-cycle cap or
/// the cached remaining-allowance prevented the call. Callers treat this as
/// "no candidates", same as an empty hit, but the matcher uses the variant
/// to write a distinct rung outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchOutcome {
	Hit,
	Miss,
	QuotaExhausted,
}

pub struct SearchResult {
	pub outcome: SearchOutcome,
	pub games: Vec<TgdbApiGame>,
}

impl TheGamesDbClient {
	pub async fn search_by_name(&self, name: &str) -> anyhow::Result<SearchResult> {
		let Some(api_key) = self.api_key.as_deref() else {
			return Ok(SearchResult {
				outcome: SearchOutcome::Miss,
				games: vec![],
			});
		};

		if !self.try_acquire_cycle_slot() {
			record_outcome("quota_exhausted");
			return Ok(SearchResult {
				outcome: SearchOutcome::QuotaExhausted,
				games: vec![],
			});
		}

		let remaining = self.load_remaining_allowance(api_key).await?;
		if remaining <= 0 {
			record_outcome("quota_exhausted");
			return Ok(SearchResult {
				outcome: SearchOutcome::QuotaExhausted,
				games: vec![],
			});
		}

		let mut url = Url::parse(&format!("{API_BASE}/v1/Games/ByGameName"))?;
		url.query_pairs_mut()
			.append_pair("apikey", api_key)
			.append_pair("name", name)
			.append_pair("fields", "")
			.append_pair("include", "alternates");

		let env: TgdbEnvelope<TgdbSearchData> = self
			.execute_get(url, "search_by_name")
			.await
			.inspect_err(|_| {
				record_outcome("error");
			})?;

		self.refresh_remaining_allowance(&env).await;

		let games = env.data.games;
		let outcome = if games.is_empty() {
			record_outcome("miss");
			SearchOutcome::Miss
		} else {
			record_outcome("hit");
			SearchOutcome::Hit
		};
		Ok(SearchResult { outcome, games })
	}

	/// Probe the API for the current `remaining_monthly_allowance`. Used to
	/// rehydrate the Redis cache when it has expired or been flushed. Hits the
	/// smallest documented endpoint (`Games/ByGameID` for id 1 with empty
	/// fields) and returns the allowance reported in the response envelope.
	async fn probe_remaining_allowance(&self, api_key: &str) -> anyhow::Result<i32> {
		let mut url = Url::parse(&format!("{API_BASE}/v1/Games/ByGameID"))?;
		url.query_pairs_mut()
			.append_pair("apikey", api_key)
			.append_pair("id", "1")
			.append_pair("fields", "");

		let env: TgdbEnvelope<TgdbGameByIdData> = self.execute_get(url, "probe").await?;
		let remaining =
			env.remaining_monthly_allowance.unwrap_or(0) + env.extra_allowance.unwrap_or(0);
		self.write_remaining_allowance(remaining).await;
		record_outcome("probe");
		Ok(remaining)
	}

	async fn load_remaining_allowance(&self, api_key: &str) -> anyhow::Result<i32> {
		let cached = self.remaining_allowance.load(Ordering::Relaxed);
		if cached != i32::MIN {
			return Ok(cached);
		}
		let mut redis_conn = self.redis_conn.clone();
		match redis_conn.get(Self::REMAINING_CACHE_KEY).await {
			Ok(Some(raw)) => match raw.parse::<i32>() {
				Ok(v) => {
					self.remaining_allowance.store(v, Ordering::Relaxed);
					crate::metrics::set_thegamesdb_remaining_allowance(v);
					Ok(v)
				}
				Err(_) => self.probe_remaining_allowance(api_key).await,
			},
			Ok(None) => self.probe_remaining_allowance(api_key).await,
			Err(e) => {
				debug!("tgdb redis quota lookup failed, probing API: {e}");
				self.probe_remaining_allowance(api_key).await
			}
		}
	}

	async fn refresh_remaining_allowance<T>(&self, env: &TgdbEnvelope<T>) {
		if let Some(remaining) = env.remaining_monthly_allowance {
			let total = remaining + env.extra_allowance.unwrap_or(0);
			self.write_remaining_allowance(total).await;
		}
	}

	async fn write_remaining_allowance(&self, value: i32) {
		self.remaining_allowance.store(value, Ordering::Relaxed);
		crate::metrics::set_thegamesdb_remaining_allowance(value);
		let mut redis_conn = self.redis_conn.clone();
		if let Err(e) = redis_conn
			.set_ex(
				Self::REMAINING_CACHE_KEY,
				value.to_string(),
				Self::REMAINING_CACHE_TTL_SECS,
			)
			.await
		{
			debug!("tgdb redis quota write failed: {e}");
		}
	}

	fn try_acquire_cycle_slot(&self) -> bool {
		let prev = self.per_cycle_calls.fetch_add(1, Ordering::Relaxed);
		if prev >= Self::PER_CYCLE_CAP {
			self.per_cycle_calls
				.store(Self::PER_CYCLE_CAP, Ordering::Relaxed);
			crate::metrics::set_thegamesdb_cycle_calls(Self::PER_CYCLE_CAP);
			false
		} else {
			crate::metrics::set_thegamesdb_cycle_calls(prev + 1);
			true
		}
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
			.http
			.request(Method::GET, url)
			.headers(headers)
			.build()?;

		debug!("tgdb request: {} {}", req.method(), url_for_log.path());

		let started = Instant::now();
		let _inflight = crate::http::abstraction::InflightGuard::new("thegamesdb");
		let mut observed_code: Option<u16> = None;
		let result: anyhow::Result<T> = async {
			let res = self.execute(req).await?;
			let status = res.status();
			observed_code = Some(status.as_u16());
			let body = res.text().await?;
			if !status.is_success() {
				return Err(anyhow!(
					"thegamesdb returned non-success status {status} from {url_for_log}"
				));
			}
			if status == StatusCode::NO_CONTENT || body.is_empty() {
				return Err(anyhow!(
					"thegamesdb returned empty body on success from {url_for_log}"
				));
			}
			serde_json::from_str::<T>(&body)
				.with_context(|| format!("failed to parse thegamesdb response from {url_for_log}"))
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
			"thegamesdb",
			endpoint_label,
			status_class,
			&status_code,
			started.elapsed().as_secs_f64(),
		);
		result
	}
}

fn record_outcome(outcome: &str) {
	crate::metrics::record_thegamesdb_api_call(outcome);
}
