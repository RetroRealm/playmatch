use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use service::error::ServiceError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("an unspecified internal error occurred: {0}")]
	InternalError(#[from] anyhow::Error),

	#[error("a database error occurred: {0}")]
	DbError(#[from] sea_orm::DbErr),

	#[error("Authentication failed: {0}")]
	InvalidAuth(String),

	#[error("You do not have permission to perform this action")]
	InvalidAuthPermission,

	#[error("User was not found")]
	UserNotFound,

	#[error("{0}")]
	BadRequest(String),

	#[error(transparent)]
	ServiceError(#[from] ServiceError),

	#[error(transparent)]
	RedisError(#[from] redis::RedisError),
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

impl ResponseError for Error {
	fn status_code(&self) -> StatusCode {
		self.status_and_metric().0
	}

	fn error_response(&self) -> HttpResponse {
		let (status, label) = self.status_and_metric();
		service::metrics::record_service_error(label);
		HttpResponse::build(status).body(self.to_string())
	}
}

pub type Result<T> = std::result::Result<T, Error>;
