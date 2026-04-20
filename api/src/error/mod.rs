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
	fn metric_variant(&self) -> &'static str {
		match self {
			Self::InternalError(_) => "internal",
			Self::DbError(_) => "db_error",
			Self::InvalidAuth(_) => "invalid_auth",
			Self::InvalidAuthPermission => "invalid_auth_permission",
			Self::UserNotFound => "user_not_found",
			Self::BadRequest(_) => "bad_request",
			Self::RedisError(_) => "redis_error",
			Self::ServiceError(err) => match err {
				ServiceError::GameNotFound => "game_not_found",
				ServiceError::PlatformNotFound => "platform_not_found",
				ServiceError::CompanyNotFound => "company_not_found",
				ServiceError::UserNotFound => "user_not_found",
				ServiceError::SuggestionAlreadyExists => "suggestion_exists",
				ServiceError::SuggestionNotFound => "suggestion_not_found",
				ServiceError::SignatureMetadataMappingInputBuilderError(_) => {
					"mapping_builder_error"
				}
				ServiceError::UpdatedMatchResultBuilderError(_) => "updated_match_builder_error",
				ServiceError::GameAndRelationsResultBuilderError(_) => {
					"game_and_relations_builder_error"
				}
				ServiceError::DbError(_) => "db_error",
				ServiceError::RedisError(_) => "redis_error",
				ServiceError::JsonError(_) => "json_error",
				ServiceError::ParseIntError(_) => "parse_int_error",
				ServiceError::AnyhowError(_) => "anyhow_error",
			},
		}
	}
}

impl ResponseError for Error {
	fn status_code(&self) -> StatusCode {
		match &self {
			Self::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
			Self::DbError(_) => StatusCode::INTERNAL_SERVER_ERROR,
			Error::ServiceError(err) => match err {
				ServiceError::GameNotFound => StatusCode::NOT_FOUND,
				ServiceError::PlatformNotFound => StatusCode::NOT_FOUND,
				ServiceError::CompanyNotFound => StatusCode::NOT_FOUND,
				ServiceError::UserNotFound => StatusCode::NOT_FOUND,
				ServiceError::SignatureMetadataMappingInputBuilderError(_) => {
					StatusCode::INTERNAL_SERVER_ERROR
				}
				ServiceError::UpdatedMatchResultBuilderError(_) => {
					StatusCode::INTERNAL_SERVER_ERROR
				}
				ServiceError::DbError(_) => StatusCode::INTERNAL_SERVER_ERROR,
				ServiceError::SuggestionAlreadyExists => StatusCode::CONFLICT,
				ServiceError::SuggestionNotFound => StatusCode::NOT_FOUND,
				ServiceError::GameAndRelationsResultBuilderError(_) => {
					StatusCode::INTERNAL_SERVER_ERROR
				}
				ServiceError::RedisError(_) => StatusCode::INTERNAL_SERVER_ERROR,
				ServiceError::JsonError(_) => StatusCode::INTERNAL_SERVER_ERROR,
				ServiceError::ParseIntError(_) => StatusCode::INTERNAL_SERVER_ERROR,
				ServiceError::AnyhowError(_) => StatusCode::INTERNAL_SERVER_ERROR,
			},
			Error::InvalidAuth(_) => StatusCode::UNAUTHORIZED,
			Error::UserNotFound => StatusCode::NOT_FOUND,
			Error::InvalidAuthPermission => StatusCode::FORBIDDEN,
			Error::BadRequest(_) => StatusCode::BAD_REQUEST,
			Error::RedisError(_) => StatusCode::INTERNAL_SERVER_ERROR,
		}
	}

	fn error_response(&self) -> HttpResponse {
		service::metrics::record_service_error(self.metric_variant());
		HttpResponse::build(self.status_code()).body(self.to_string())
	}
}

// Short hand alias, which allows you to use just Result<T>
pub type Result<T> = std::result::Result<T, Error>;
