use crate::model::game_file::GameMatchResponseBuilderError;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("an unspecified internal error occurred: {0}")]
	InternalError(#[from] anyhow::Error),

	#[error("a database error occurred: {0}")]
	DbError(#[from] sea_orm::DbErr),

	#[error("a response builder error occurred: {0}")]
	BuilderError(#[from] GameMatchResponseBuilderError),
}

impl ResponseError for Error {
	fn status_code(&self) -> StatusCode {
		match &self {
			Self::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
			Self::DbError(_) => StatusCode::INTERNAL_SERVER_ERROR,
			Error::BuilderError(_) => StatusCode::INTERNAL_SERVER_ERROR,
		}
	}

	fn error_response(&self) -> HttpResponse {
		HttpResponse::build(self.status_code()).body(self.to_string())
	}
}

// Short hand alias, which allows you to use just Result<T>
pub type Result<T> = std::result::Result<T, Error>;
