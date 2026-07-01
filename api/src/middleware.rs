use actix_web::Error;
use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::http::StatusCode;
use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::middleware::Next;
use chrono::{NaiveDate, TimeZone, Utc};
use service::config::versions::{
	ApiVersion, ApiVersionInfo, VersionStatus, latest_version, version_info,
};
use service::metrics::{
	http_requests_inflight_dec, http_requests_inflight_inc, record_http_rate_limit_rejected,
	record_user_agent,
};

/// Unversioned discovery document every API response points at.
const VERSION_DISCOVERY_PATH: &str = "/api/versions";

/// Namespaced link relation for the discovery document. A bare token like
/// `version-history` is not IANA-registered, so RFC 8288 requires an extension
/// rel be expressed as a URI.
const VERSION_HISTORY_REL: &str = "https://playmatch.retrorealm.dev/rel/version-history";

/// Always-on `Link` header pointing every API response at the discovery doc.
fn discovery_link_header() -> (HeaderName, HeaderValue) {
	let value = format!("<{VERSION_DISCOVERY_PATH}>; rel=\"{VERSION_HISTORY_REL}\"");
	(
		HeaderName::from_static("link"),
		HeaderValue::from_str(&value).expect("discovery link header is valid ascii"),
	)
}

/// Builds the conditional lifecycle headers for a version. Returns an empty
/// vector while the version is active so the plumbing stays inert until a
/// version is actually deprecated. Deprecation uses the RFC 9745 `@<epoch>`
/// form; Sunset uses the RFC 8594 HTTP-date; the successor `Link` uses the
/// RFC 5829 `successor-version` relation.
fn lifecycle_headers(info: &ApiVersionInfo) -> Vec<(HeaderName, String)> {
	if info.status == VersionStatus::Active {
		return Vec::new();
	}

	let mut headers = Vec::new();

	if let Some(epoch) = info.deprecated_on.and_then(parse_date_to_epoch) {
		headers.push((HeaderName::from_static("deprecation"), format!("@{epoch}")));
	}

	if let Some(http_date) = info.sunset_on.and_then(parse_date_to_http_date) {
		headers.push((HeaderName::from_static("sunset"), http_date));
	}

	let successor = latest_version();
	if successor != info.version {
		headers.push((
			HeaderName::from_static("link"),
			format!("</api/{}>; rel=\"successor-version\"", successor.as_str()),
		));
	}

	headers
}

fn parse_date_to_epoch(raw: &str) -> Option<i64> {
	let date = NaiveDate::parse_from_str(raw, "%Y-%m-%d").ok()?;
	let datetime = date.and_hms_opt(0, 0, 0)?;
	Some(Utc.from_utc_datetime(&datetime).timestamp())
}

fn parse_date_to_http_date(raw: &str) -> Option<String> {
	let date = NaiveDate::parse_from_str(raw, "%Y-%m-%d").ok()?;
	let datetime = date.and_hms_opt(0, 0, 0)?;
	Some(
		Utc.from_utc_datetime(&datetime)
			.format("%a, %d %b %Y %H:%M:%S GMT")
			.to_string(),
	)
}

type LifecycleFuture<B> =
	std::pin::Pin<Box<dyn std::future::Future<Output = Result<ServiceResponse<B>, Error>>>>;

/// Per version-scope middleware that stamps the always-on discovery `Link` and,
/// once a version leaves `active`, the RFC 9745 / 8594 / 5829 deprecation
/// signals. Inert for active versions beyond the discovery link.
pub fn version_lifecycle_headers<B: MessageBody + 'static>(
	version: ApiVersion,
) -> impl Fn(ServiceRequest, Next<B>) -> LifecycleFuture<B> + Clone {
	move |req: ServiceRequest, next: Next<B>| {
		Box::pin(async move {
			let mut res = next.call(req).await?;
			let headers = res.headers_mut();

			let (name, value) = discovery_link_header();
			headers.append(name, value);

			if let Some(info) = version_info(version) {
				for (name, raw) in lifecycle_headers(info) {
					if let Ok(value) = HeaderValue::from_str(&raw) {
						headers.append(name, value);
					}
				}
			}

			Ok(res)
		})
	}
}

type EnvelopeFuture =
	std::pin::Pin<Box<dyn std::future::Future<Output = Result<ServiceResponse, Error>>>>;

/// Per version-scope middleware that re-renders every error response as the v2
/// JSON envelope `{code, message}`. v1 (and any future frozen version) passes
/// through untouched so its plain-text bodies stay byte-identical. A typed
/// [`crate::error::Error`] keeps its stable metric label as the `code`; any other
/// error status (a failed extractor, a rate-limit 429, an unmatched-route 404, a
/// wrong-method 405) is mapped by status.
///
/// It wraps OUTSIDE the rate limiter so the Governor's 429 and the scope's
/// own 404/405 are also converted, and preserves the original response headers
/// (`Retry-After`, the rate-limit headers, `Cache-Control`, `Vary`) that a naive
/// body swap would drop.
pub fn v2_error_envelope<B: MessageBody + 'static>(
	version: ApiVersion,
) -> impl Fn(ServiceRequest, Next<B>) -> EnvelopeFuture + Clone {
	move |req: ServiceRequest, next: Next<B>| {
		Box::pin(async move {
			let res = match next.call(req).await {
				Ok(res) => res,
				Err(err) if version == ApiVersion::V1 => return Err(err),
				Err(err) => return Err(rerender_actix_error(err)),
			};

			if version == ApiVersion::V1 {
				return Ok(res.map_into_boxed_body());
			}

			match v2_replacement(&res) {
				Some(mut replacement) => {
					copy_surviving_headers(res.response().headers(), &mut replacement);
					Ok(res.into_response(replacement))
				}
				None => Ok(res.map_into_boxed_body()),
			}
		})
	}
}

fn response_is_json(headers: &actix_web::http::header::HeaderMap) -> bool {
	headers
		.get(actix_web::http::header::CONTENT_TYPE)
		.and_then(|v| v.to_str().ok())
		.map(|v| v.starts_with("application/json"))
		.unwrap_or(false)
}

/// Carry the original response headers onto the envelope replacement, skipping the
/// content headers the JSON body owns and any header the replacement already set.
/// Keeps `Retry-After` and the rate-limit headers on a rewritten 429, and the
/// `Cache-Control`/`Vary` an inner authenticated scope stamped on an auth error.
fn copy_surviving_headers(
	original: &actix_web::http::header::HeaderMap,
	replacement: &mut actix_web::HttpResponse,
) {
	use actix_web::http::header::{CONTENT_LENGTH, CONTENT_TYPE};
	for (name, value) in original {
		if name == CONTENT_TYPE || name == CONTENT_LENGTH {
			continue;
		}
		if !replacement.headers().contains_key(name) {
			replacement
				.headers_mut()
				.append(name.clone(), value.clone());
		}
	}
}

/// The v2-envelope replacement for an error response, or `None` to leave a
/// success/redirect or already-JSON response untouched. Every 4xx/5xx that is not
/// already JSON is rewritten so the v2 surface never emits a non-JSON error.
/// Metrics were already recorded when the error was first rendered, so the typed
/// re-render deliberately does not record again.
fn v2_replacement<B>(res: &ServiceResponse<B>) -> Option<actix_web::HttpResponse> {
	let resp = res.response();
	let status = resp.status();
	if !(status.is_client_error() || status.is_server_error()) {
		return None;
	}
	if response_is_json(resp.headers()) {
		return None;
	}
	match resp
		.error()
		.and_then(|e| e.as_error::<crate::error::Error>())
	{
		Some(typed) => Some(typed.v2_error_response()),
		None => Some(framework_error_to_v2(status, resp.error())),
	}
}

/// Re-render an error that propagated as `Err` (the uncommon path) into the v2
/// envelope. This error has not been rendered yet, so it is recorded here.
fn rerender_actix_error(err: Error) -> Error {
	let response = match err.as_error::<crate::error::Error>() {
		Some(typed) => {
			typed.record_error_metrics();
			typed.v2_error_response()
		}
		None => {
			let rendered = err.error_response();
			if response_is_json(rendered.headers()) {
				rendered
			} else {
				framework_error_to_v2(rendered.status(), Some(&err))
			}
		}
	};
	actix_web::error::InternalError::from_response(err, response).into()
}

/// Map a non-typed error status (extractor failure, rate limit, unmatched route,
/// and so on) to the v2 envelope with a stable `code`. Client-error messages are
/// echoed when available; server errors collapse to a generic message.
fn framework_error_to_v2(status: StatusCode, err: Option<&Error>) -> actix_web::HttpResponse {
	let code = match status {
		StatusCode::BAD_REQUEST => "bad_request",
		StatusCode::UNAUTHORIZED => "unauthorized",
		StatusCode::FORBIDDEN => "forbidden",
		StatusCode::NOT_FOUND => "not_found",
		StatusCode::METHOD_NOT_ALLOWED => "method_not_allowed",
		StatusCode::PAYLOAD_TOO_LARGE => "payload_too_large",
		StatusCode::UNSUPPORTED_MEDIA_TYPE => "unsupported_media_type",
		StatusCode::TOO_MANY_REQUESTS => "rate_limited",
		StatusCode::SERVICE_UNAVAILABLE => "service_unavailable",
		s if s.is_server_error() => "internal_error",
		_ => "error",
	};
	let message = if status.is_client_error() {
		err.map(|e| e.to_string())
			.unwrap_or_else(|| status.canonical_reason().unwrap_or("error").to_string())
	} else {
		"internal server error".to_string()
	};
	crate::routes::v2::error::v2_status_error(crate::error::RenderParts {
		status,
		code,
		message,
		retry_after_secs: None,
	})
}

pub async fn http_request_metrics<B: MessageBody>(
	req: ServiceRequest,
	next: Next<B>,
) -> Result<ServiceResponse<B>, Error> {
	let ua = req
		.headers()
		.get("User-Agent")
		.and_then(|h| h.to_str().ok())
		.unwrap_or("");
	let (product, version) = classify_user_agent(ua);
	record_user_agent(product, &version);

	let route = req.match_pattern().unwrap_or_else(|| "UNKNOWN".to_string());
	let method = req.method().as_str().to_string();
	let _guard = InflightGuard::new(route, method);

	let res = next.call(req).await?;
	if res.status() == StatusCode::TOO_MANY_REQUESTS {
		record_http_rate_limit_rejected(product);
	}
	Ok(res)
}

struct InflightGuard {
	route: String,
	method: String,
}

impl InflightGuard {
	fn new(route: String, method: String) -> Self {
		http_requests_inflight_inc(&route, &method);
		Self { route, method }
	}
}

impl Drop for InflightGuard {
	fn drop(&mut self) {
		http_requests_inflight_dec(&self.route, &self.method);
	}
}

fn classify_user_agent(ua: &str) -> (&'static str, String) {
	if ua.is_empty() {
		return ("none", String::new());
	}

	if let Some(rest) = ua.strip_prefix("RomM/") {
		let raw = rest
			.split(|c: char| c.is_whitespace() || c == ';' || c == ',')
			.next()
			.unwrap_or("");
		let version = if is_safe_version(raw) {
			raw.to_string()
		} else {
			"unknown".to_string()
		};
		return ("romm", version);
	}

	let ua_lower = ua.to_ascii_lowercase();
	if ua_lower.contains("bot")
		|| ua_lower.contains("crawler")
		|| ua_lower.contains("spider")
		|| ua_lower.contains("scraper")
	{
		return ("bot", String::new());
	}

	if ua.starts_with("curl/") {
		return ("curl", String::new());
	}

	if ua.starts_with("Mozilla/") {
		if ua.contains("Firefox/") {
			return ("browser", "firefox".to_string());
		}
		if ua.contains("Edg/") {
			return ("browser", "edge".to_string());
		}
		if ua.contains("Chrome/") {
			return ("browser", "chrome".to_string());
		}
		if ua.contains("Safari/") {
			return ("browser", "safari".to_string());
		}
		return ("browser", "other".to_string());
	}

	("other", String::new())
}

fn is_safe_version(v: &str) -> bool {
	!v.is_empty()
		&& v.len() <= 32
		&& v.chars()
			.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+' | '_'))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn empty_ua_classifies_as_none() {
		assert_eq!(classify_user_agent(""), ("none", String::new()));
	}

	#[test]
	fn romm_version_is_extracted() {
		assert_eq!(
			classify_user_agent("RomM/3.5.0"),
			("romm", "3.5.0".to_string())
		);
		assert_eq!(
			classify_user_agent("RomM/3.5.0-beta.1"),
			("romm", "3.5.0-beta.1".to_string())
		);
	}

	#[test]
	fn romm_version_with_suffix_is_cleaned() {
		assert_eq!(
			classify_user_agent("RomM/3.5.0 (Python/3.11)"),
			("romm", "3.5.0".to_string())
		);
	}

	#[test]
	fn romm_garbage_version_buckets_to_unknown() {
		let garbage = "RomM/".to_string() + &"x".repeat(100);
		assert_eq!(
			classify_user_agent(&garbage),
			("romm", "unknown".to_string())
		);
		assert_eq!(
			classify_user_agent("RomM/<script>"),
			("romm", "unknown".to_string())
		);
	}

	#[test]
	fn bot_user_agents_are_bucketed() {
		assert_eq!(
			classify_user_agent(
				"Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)"
			),
			("bot", String::new())
		);
		assert_eq!(
			classify_user_agent("SemrushBot/7~bl"),
			("bot", String::new())
		);
	}

	#[test]
	fn curl_is_bucketed_without_version() {
		assert_eq!(classify_user_agent("curl/8.4.0"), ("curl", String::new()));
	}

	#[test]
	fn browsers_are_bucketed() {
		assert_eq!(
			classify_user_agent(
				"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
			),
			("browser", "chrome".to_string())
		);
		assert_eq!(
			classify_user_agent(
				"Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/117.0"
			),
			("browser", "firefox".to_string())
		);
	}

	#[test]
	fn unknown_ua_buckets_to_other() {
		assert_eq!(
			classify_user_agent("SomeCustomClient 1.0"),
			("other", String::new())
		);
	}

	#[test]
	fn active_version_emits_no_lifecycle_headers() {
		let info = ApiVersionInfo {
			version: ApiVersion::V1,
			status: VersionStatus::Active,
			is_default: true,
			released_on: Some("2024-01-01"),
			deprecated_on: None,
			sunset_on: None,
			removed_on: None,
			docs_url: None,
			openapi_url: None,
		};
		assert!(
			lifecycle_headers(&info).is_empty(),
			"the plumbing must stay inert while a version is active"
		);
	}

	#[test]
	fn deprecated_version_emits_rfc_signals() {
		let info = ApiVersionInfo {
			version: ApiVersion::V1,
			status: VersionStatus::Deprecated,
			is_default: false,
			released_on: Some("2024-01-01"),
			deprecated_on: Some("2030-01-01"),
			sunset_on: Some("2031-06-30"),
			removed_on: None,
			docs_url: None,
			openapi_url: None,
		};

		let headers = lifecycle_headers(&info);
		let deprecation = headers
			.iter()
			.find(|(n, _)| n.as_str() == "deprecation")
			.map(|(_, v)| v.clone());
		assert_eq!(
			deprecation.as_deref(),
			Some("@1893456000"),
			"RFC 9745 deprecation must be the @epoch of 2030-01-01 UTC"
		);

		let sunset = headers
			.iter()
			.find(|(n, _)| n.as_str() == "sunset")
			.map(|(_, v)| v.clone());
		assert_eq!(
			sunset.as_deref(),
			Some("Mon, 30 Jun 2031 00:00:00 GMT"),
			"RFC 8594 sunset must be an HTTP-date"
		);

		let link = headers
			.iter()
			.find(|(n, _)| n.as_str() == "link")
			.map(|(_, v)| v.clone());
		assert_eq!(
			link.as_deref(),
			Some("</api/v2>; rel=\"successor-version\""),
			"the successor link must point at the latest version"
		);
	}

	#[test]
	fn discovery_link_is_namespaced() {
		let (name, value) = discovery_link_header();
		assert_eq!(name.as_str(), "link");
		let rendered = value.to_str().unwrap();
		assert!(rendered.starts_with("</api/versions>;"));
		assert!(
			rendered.contains("https://playmatch.retrorealm.dev/rel/version-history"),
			"the discovery rel must be a namespaced URI, got {rendered}"
		);
	}
}
