use crate::config::http::REQWEST_DEFAULT_USER_AGENT;
use log::{error, warn};
use rand::RngExt;
use reqwest::{IntoUrl, Request, RequestBuilder, Response};
use std::time::Duration;
use tower::retry::Policy;

pub const MAX_RETRIES: usize = 3;
const BACKOFF_MS: &[u64] = &[250, 500, 1000];
const JITTER_MAX_MS: u64 = 100;

/// Maps a numeric HTTP status code to a coarse class label suitable for a
/// Prometheus label value.
pub fn classify_status(code: u16) -> &'static str {
	match code {
		100..=199 => "1xx",
		200..=299 => "2xx",
		300..=399 => "3xx",
		400..=499 => "4xx",
		_ => "5xx",
	}
}

/// Maps an outbound HTTP outcome to a Prometheus-friendly `(status_class,
/// status_code)` pair. Status code is "none" when no response landed (the
/// request errored before a server reply).
pub fn classify_http_outcome(result: &Result<Response, reqwest::Error>) -> (&'static str, String) {
	match result {
		Ok(res) => {
			let code = res.status().as_u16();
			(classify_status(code), code.to_string())
		}
		Err(e) if e.is_timeout() => ("timeout", "none".to_string()),
		Err(_) => ("network_error", "none".to_string()),
	}
}

#[derive(Debug, Clone)]
pub struct RetryPolicy {
	provider: &'static str,
	remaining: usize,
	max: usize,
}

impl RetryPolicy {
	pub fn new(provider: &'static str) -> Self {
		Self::with_max(provider, MAX_RETRIES)
	}

	pub fn with_max(provider: &'static str, max: usize) -> Self {
		Self {
			provider,
			remaining: max,
			max,
		}
	}
}

#[derive(Debug, PartialEq, Eq)]
enum RetryDecision {
	Skip,
	Retry { delay_ms: u64 },
	Exhausted,
}

fn decide_retry(remaining: usize, max: usize, is_retryable: bool) -> RetryDecision {
	if !is_retryable {
		return RetryDecision::Skip;
	}
	if remaining == 0 {
		return RetryDecision::Exhausted;
	}
	let attempt_done = max - remaining + 1;
	let base = BACKOFF_MS[(attempt_done - 1).min(BACKOFF_MS.len() - 1)];
	let jitter: u64 = rand::rng().random_range(0..=JITTER_MAX_MS);
	RetryDecision::Retry {
		delay_ms: base + jitter,
	}
}

impl<E: std::fmt::Display> Policy<Request, Response, E> for RetryPolicy {
	type Future = tokio::time::Sleep;

	fn retry(
		&mut self,
		req: &mut Request,
		result: &mut Result<Response, E>,
	) -> Option<Self::Future> {
		let is_retryable = match result {
			Err(_) => true,
			Ok(res) => res.status().is_server_error(),
		};
		let attempt_done = self.max - self.remaining + 1;
		match decide_retry(self.remaining, self.max, is_retryable) {
			RetryDecision::Skip => None,
			RetryDecision::Exhausted => {
				let cause = match result {
					Err(e) => format!("network error: {e}"),
					Ok(res) => format!("HTTP {}", res.status().as_u16()),
				};
				error!(
					"{} request to {} failed after {} retries ({cause})",
					self.provider,
					req.url(),
					self.max
				);
				None
			}
			RetryDecision::Retry { delay_ms } => {
				let cause = match result {
					Err(e) => format!("network error: {e}"),
					Ok(res) => format!("HTTP {}", res.status().as_u16()),
				};
				warn!(
					"{} request to {} returned {cause}; retrying ({attempt_done}/{}) after {delay_ms}ms",
					self.provider,
					req.url(),
					self.max
				);
				self.remaining -= 1;
				Some(tokio::time::sleep(Duration::from_millis(delay_ms)))
			}
		}
	}

	fn clone_request(&mut self, req: &Request) -> Option<Request> {
		req.try_clone()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn skips_when_not_retryable() {
		assert_eq!(decide_retry(3, 3, false), RetryDecision::Skip);
	}

	#[test]
	fn exhausted_when_remaining_is_zero() {
		assert_eq!(decide_retry(0, 3, true), RetryDecision::Exhausted);
	}

	#[test]
	fn retries_when_remaining_and_retryable() {
		match decide_retry(3, 3, true) {
			RetryDecision::Retry { delay_ms } => {
				assert!((BACKOFF_MS[0]..=BACKOFF_MS[0] + JITTER_MAX_MS).contains(&delay_ms));
			}
			other => panic!("expected Retry, got {other:?}"),
		}
	}

	#[test]
	fn backoff_grows_with_attempts() {
		fn base_for(remaining: usize, max: usize) -> u64 {
			let attempt_done = max - remaining + 1;
			BACKOFF_MS[(attempt_done - 1).min(BACKOFF_MS.len() - 1)]
		}
		assert_eq!(base_for(3, 3), 250);
		assert_eq!(base_for(2, 3), 500);
		assert_eq!(base_for(1, 3), 1000);
	}

	#[test]
	fn backoff_clamps_to_last_step_after_schedule_ends() {
		fn base_for(remaining: usize, max: usize) -> u64 {
			let attempt_done = max - remaining + 1;
			BACKOFF_MS[(attempt_done - 1).min(BACKOFF_MS.len() - 1)]
		}
		assert_eq!(base_for(4, 5), 500);
		assert_eq!(base_for(3, 5), 1000);
		assert_eq!(base_for(2, 5), 1000);
		assert_eq!(base_for(1, 5), 1000);
	}

	#[test]
	fn classify_status_buckets_known_codes() {
		assert_eq!(classify_status(100), "1xx");
		assert_eq!(classify_status(200), "2xx");
		assert_eq!(classify_status(204), "2xx");
		assert_eq!(classify_status(301), "3xx");
		assert_eq!(classify_status(404), "4xx");
		assert_eq!(classify_status(429), "4xx");
		assert_eq!(classify_status(500), "5xx");
		assert_eq!(classify_status(503), "5xx");
	}

	#[test]
	fn classify_status_treats_unknown_high_codes_as_server_error() {
		assert_eq!(classify_status(600), "5xx");
		assert_eq!(classify_status(999), "5xx");
	}
}

pub trait RequestClientExt {
	fn get_default_user_agent<U: IntoUrl>(&self, url: U) -> RequestBuilder;
}

impl RequestClientExt for reqwest::Client {
	fn get_default_user_agent<U: IntoUrl>(&self, url: U) -> RequestBuilder {
		self.get(url)
			.header("User-Agent", REQWEST_DEFAULT_USER_AGENT)
	}
}
