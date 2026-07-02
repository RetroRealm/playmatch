use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use service::error::ServiceError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("an unspecified internal error occurred: {0}")]
	InternalError(#[from] anyhow::Error),

	#[error("a database error occurred: {0}")]
	DbError(anyhow::Error),

	#[error("authentication failed: {0}")]
	InvalidAuth(String),

	#[error("insufficient permissions for this action")]
	InvalidAuthPermission,

	#[error("user not found")]
	UserNotFound,

	#[error("{0}")]
	BadRequest(String),

	#[error(transparent)]
	ServiceError(#[from] ServiceError),

	#[error("{0}")]
	RedisError(anyhow::Error),

	#[error("{message}")]
	UpstreamUnavailable {
		provider: &'static str,
		message: String,
		retry_after_secs: Option<u64>,
	},
}

impl From<sea_orm::DbErr> for Error {
	fn from(e: sea_orm::DbErr) -> Self {
		Self::DbError(anyhow::Error::from(e))
	}
}

impl From<redis::RedisError> for Error {
	fn from(e: redis::RedisError) -> Self {
		Self::RedisError(anyhow::Error::from(e))
	}
}

impl Error {
	fn status_and_metric(&self) -> (StatusCode, &'static str) {
		match self {
			Self::InternalError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal"),
			Self::DbError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "db_error"),
			Self::InvalidAuth(_) => (StatusCode::UNAUTHORIZED, "invalid_auth"),
			Self::InvalidAuthPermission => (StatusCode::FORBIDDEN, "invalid_auth_permission"),
			Self::UserNotFound => (StatusCode::NOT_FOUND, "user_not_found"),
			Self::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
			Self::RedisError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "redis_error"),
			Self::UpstreamUnavailable { .. } => {
				(StatusCode::SERVICE_UNAVAILABLE, "upstream_unavailable")
			}
			Self::ServiceError(err) => match err {
				ServiceError::GameNotFound => (StatusCode::NOT_FOUND, "game_not_found"),
				ServiceError::PlatformNotFound => (StatusCode::NOT_FOUND, "platform_not_found"),
				ServiceError::CompanyNotFound => (StatusCode::NOT_FOUND, "company_not_found"),
				ServiceError::UserNotFound => (StatusCode::NOT_FOUND, "user_not_found"),
				ServiceError::SuggestionAlreadyExists => {
					(StatusCode::CONFLICT, "suggestion_exists")
				}
				ServiceError::SuggestionNotFound => (StatusCode::NOT_FOUND, "suggestion_not_found"),
				ServiceError::SignatureMetadataMappingInputBuilderError(_) => {
					(StatusCode::INTERNAL_SERVER_ERROR, "mapping_builder_error")
				}
				ServiceError::UpdatedMatchResultBuilderError(_) => (
					StatusCode::INTERNAL_SERVER_ERROR,
					"updated_match_builder_error",
				),
				ServiceError::GameAndRelationsResultBuilderError(_) => (
					StatusCode::INTERNAL_SERVER_ERROR,
					"game_and_relations_builder_error",
				),
				ServiceError::DbError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "db_error"),
				ServiceError::RedisError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "redis_error"),
				ServiceError::JsonError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "json_error"),
				ServiceError::ParseIntError(_) => {
					(StatusCode::INTERNAL_SERVER_ERROR, "parse_int_error")
				}
				ServiceError::AnyhowError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "anyhow_error"),
			},
		}
	}
}

/// The pieces both error renderers need, produced once per rendered error so
/// metrics are recorded exactly once regardless of which surface renders it.
pub(crate) struct RenderParts {
	pub status: StatusCode,
	pub code: &'static str,
	pub message: String,
	pub retry_after_secs: Option<u64>,
}

impl Error {
	/// Records the error metrics and logs server errors. Kept separate from
	/// rendering so the v2 middleware can re-render an already-recorded error into
	/// the JSON envelope without double-counting. Call exactly once per response.
	pub(crate) fn record_error_metrics(&self) {
		let (status, code) = self.status_and_metric();
		service::metrics::record_service_error(code);
		self.record_extra_metrics();
		if status.is_server_error() && !matches!(self, Self::UpstreamUnavailable { .. }) {
			log::error!("HTTP {} ({code}): {self}", status.as_u16());
		}
	}

	/// The render inputs, without recording metrics. The message is leak-safe:
	/// client errors and the deliberate upstream-unavailable case carry their real
	/// message, every other server error collapses to a generic string.
	pub(crate) fn render_parts(&self) -> RenderParts {
		let (status, code) = self.status_and_metric();
		let leak_safe =
			!status.is_server_error() || matches!(self, Self::UpstreamUnavailable { .. });

		let message = if leak_safe {
			self.to_string()
		} else {
			"internal server error".to_string()
		};

		let retry_after_secs = match self {
			Self::UpstreamUnavailable {
				retry_after_secs, ..
			} => *retry_after_secs,
			_ => None,
		};

		RenderParts {
			status,
			code,
			message,
			retry_after_secs,
		}
	}

	/// Render this error as the v2 JSON envelope (`{code, message, ...}`) instead of
	/// v1's plain-text body. Pure render, no metrics; the caller records once.
	pub(crate) fn v2_error_response(&self) -> HttpResponse {
		crate::routes::v2::error::v2_status_error(self.render_parts())
	}
}

impl ResponseError for Error {
	fn status_code(&self) -> StatusCode {
		self.status_and_metric().0
	}

	fn error_response(&self) -> HttpResponse {
		self.record_error_metrics();
		let parts = self.render_parts();

		let mut builder = HttpResponse::build(parts.status);
		if let Some(s) = parts.retry_after_secs {
			builder.insert_header(("Retry-After", s.to_string()));
		}
		builder.body(parts.message)
	}
}

impl Error {
	fn record_extra_metrics(&self) {
		match self {
			Self::UpstreamUnavailable { provider, .. } => {
				service::metrics::record_upstream_unavailable(provider);
			}
			Self::ServiceError(ServiceError::SignatureMetadataMappingInputBuilderError(e)) => {
				service::metrics::record_builder_error("mapping", &builder_failed_field(e));
			}
			Self::ServiceError(ServiceError::UpdatedMatchResultBuilderError(e)) => {
				service::metrics::record_builder_error("updated_match", &builder_failed_field(e));
			}
			Self::ServiceError(ServiceError::GameAndRelationsResultBuilderError(e)) => {
				service::metrics::record_builder_error(
					"game_and_relations",
					&builder_failed_field(e),
				);
			}
			_ => {}
		}
	}
}

/// derive_builder's UninitializedField Display is `Field not initialized: foo`.
/// Extract `foo` for the Prometheus label; fall back to `unknown` for any other
/// builder error shape so cardinality stays bounded.
fn builder_failed_field(err: &dyn std::fmt::Display) -> String {
	let msg = err.to_string();
	if let Some(rest) = msg.strip_prefix("Field not initialized: ") {
		return rest.trim().to_string();
	}
	if let Some((_, after)) = msg.split_once(':') {
		return after.trim().to_string();
	}
	"unknown".to_string()
}

pub type Result<T> = std::result::Result<T, Error>;
