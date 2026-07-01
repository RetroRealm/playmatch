//! The single machine-readable error envelope for every v2 4xx response. v1
//! keeps its plain-text bodies via the shared `crate::error::Error`; this type is
//! v2-only so the two surfaces never share a body shape.

use actix_web::HttpResponse;
use actix_web::http::StatusCode;
use actix_web::http::header::{HeaderName, HeaderValue};
use serde::Serialize;
use service::error::ServiceError;
use utoipa::ToSchema;

/// Serialize `Some` as a 200 JSON body, or short-circuit a missing read with the
/// v2 [`V2ErrorBody`] 404 envelope carrying `code`/`message`. The v2 counterpart
/// of `crate::routes::ok_or_not_found`, whose empty-body 404 the v2 surface must
/// not emit. Used by the v2 singular reads and reverse lookups so every v2 read
/// 404 is the same machine-readable JSON shape.
pub fn ok_or_v2_not_found<T: Serialize>(
	opt: Option<T>,
	code: &'static str,
	message: impl Into<String>,
) -> HttpResponse {
	match opt {
		Some(value) => HttpResponse::Ok().json(value),
		None => v2_not_found(code, message),
	}
}

/// Serialize `Ok` as a 200 JSON body and turn the service layer's
/// [`ServiceError::GameNotFound`] into the v2 [`V2ErrorBody`] 404 envelope. Every
/// other error stays an `error::Error` so it still flows through the shared
/// `ResponseError` (5xx, auth, and so on); only the game-not-found path is
/// rewritten so the v2 reads that resolve a game through the service layer answer
/// the same JSON 404 as the rest of the v2 surface instead of the shared
/// plain-text body.
pub fn game_ok_or_v2_not_found<T: Serialize>(
	result: Result<T, ServiceError>,
) -> crate::error::Result<HttpResponse> {
	match result {
		Ok(value) => Ok(HttpResponse::Ok().json(value)),
		Err(ServiceError::GameNotFound) => Ok(v2_not_found("game_not_found", "game not found")),
		Err(err) => Err(err.into()),
	}
}

/// A v2 client error. Every v2 4xx response serializes to this shape with a
/// stable `code` the client can branch on and a human-readable `message`. The
/// optional fields are populated only by the error variants that carry them, so
/// the common body stays `{ "code": ..., "message": ... }`.
#[derive(Debug, Serialize, ToSchema)]
pub struct V2ErrorBody {
	pub code: &'static str,

	pub message: String,

	/// Set on a cursor filter-set mismatch to tell the client to discard the
	/// cursor and request the first page.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub restart: Option<bool>,

	/// The maximum number of items a bulk request may carry. Present on a
	/// batch-too-large error.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub limit: Option<usize>,

	/// The number of items the rejected bulk request carried. Present on a
	/// batch-too-large error.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub received: Option<usize>,
}

impl V2ErrorBody {
	fn new(code: &'static str, message: impl Into<String>) -> Self {
		Self {
			code,
			message: message.into(),
			restart: None,
			limit: None,
			received: None,
		}
	}

	fn into_response(self, status: StatusCode) -> HttpResponse {
		HttpResponse::build(status).json(self)
	}
}

/// A 400 carrying just `code` and `message`.
pub fn v2_bad_request(code: &'static str, message: impl Into<String>) -> HttpResponse {
	V2ErrorBody::new(code, message).into_response(StatusCode::BAD_REQUEST)
}

/// A 400 that asks the client to restart paging from page 1. The `restart` flag
/// is what tells a paging client to drop its stale cursor rather than treat the
/// 400 as a hard failure.
pub fn v2_cursor_restart(code: &'static str, message: impl Into<String>) -> HttpResponse {
	let mut body = V2ErrorBody::new(code, message);
	body.restart = Some(true);
	body.into_response(StatusCode::BAD_REQUEST)
}

/// A 400 for a bulk batch rejected on size, carrying the cap and the received
/// count alongside the common `code`/`message`.
pub fn v2_batch_error(
	code: &'static str,
	message: impl Into<String>,
	limit: usize,
	received: usize,
) -> HttpResponse {
	let mut body = V2ErrorBody::new(code, message);
	body.limit = Some(limit);
	body.received = Some(received);
	body.into_response(StatusCode::BAD_REQUEST)
}

/// The 400 a bulk endpoint returns when the batch exceeds the item cap. Carries
/// the structured `batch_too_large` body plus an `x-bulk-max-items` header echoing
/// the cap so a client can read the limit without parsing the body. The header is
/// built from the numeric cap, which is always a valid header value.
pub fn batch_too_large_response(limit: usize, received: usize) -> HttpResponse {
	let mut resp = v2_batch_error(
		"batch_too_large",
		format!("batch exceeds the {limit}-item cap"),
		limit,
		received,
	);
	resp.headers_mut().insert(
		HeaderName::from_static("x-bulk-max-items"),
		HeaderValue::from(limit as u64),
	);
	resp
}

/// A 404 carrying just `code` and `message`.
pub fn v2_not_found(code: &'static str, message: impl Into<String>) -> HttpResponse {
	V2ErrorBody::new(code, message).into_response(StatusCode::NOT_FOUND)
}

/// Render a shared [`crate::error::Error`] as the v2 envelope at any status, using
/// the error's stable metric label as the machine-readable `code`. The v2 error
/// middleware calls this so a reused v1 handler, an auth failure, or a service
/// error answers `{code, message}` on the v2 surface instead of v1's plain text.
/// Carries the `Retry-After` header for the upstream-unavailable case.
pub(crate) fn v2_status_error(parts: crate::error::RenderParts) -> HttpResponse {
	let mut resp = V2ErrorBody::new(parts.code, parts.message).into_response(parts.status);
	if let Some(secs) = parts.retry_after_secs {
		resp.headers_mut().insert(
			HeaderName::from_static("retry-after"),
			HeaderValue::from(secs),
		);
	}
	resp
}

/// The 400 a v2 endpoint returns when a request body cannot be parsed: malformed
/// JSON, the wrong content type, or a body over the size cap. Wired into the v2
/// scope's `JsonConfig` error handler so a bad bulk body answers the v2 envelope
/// instead of actix's default plain-text body, which would otherwise reach the
/// client unchanged because it never passes through a v2 handler.
pub fn v2_malformed_body(message: impl Into<String>) -> HttpResponse {
	v2_bad_request("malformed_body", message)
}
