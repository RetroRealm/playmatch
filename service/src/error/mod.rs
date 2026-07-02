use crate::db::signature_metadata_mapping::SignatureMetadataMappingInputBuilderError;

#[derive(thiserror::Error, Debug)]
pub enum ServiceError {
	#[error("no game found for the given hashes or file name")]
	GameNotFound,

	#[error("no platform found with the given name")]
	PlatformNotFound,

	#[error("no company found with the given name")]
	CompanyNotFound,

	#[error("no user found with the given id")]
	UserNotFound,

	#[error(
		"a suggestion for this game, platform or company with the same provider and provider id already exists"
	)]
	SuggestionAlreadyExists,

	#[error("no suggestion found with the given id")]
	SuggestionNotFound,

	#[error(transparent)]
	SignatureMetadataMappingInputBuilderError(#[from] SignatureMetadataMappingInputBuilderError),

	#[error(transparent)]
	UpdatedMatchResultBuilderError(#[from] crate::model::UpdatedMatchResultBuilderError),

	#[error(transparent)]
	GameAndRelationsResultBuilderError(#[from] crate::model::GameAndRelationsResultBuilderError),

	#[error(transparent)]
	DbError(#[from] sea_orm::DbErr),

	#[error(transparent)]
	RedisError(#[from] redis::RedisError),

	#[error(transparent)]
	JsonError(#[from] serde_json::Error),

	#[error(transparent)]
	ParseIntError(#[from] std::num::ParseIntError),

	#[error(transparent)]
	AnyhowError(#[from] anyhow::Error),
}

pub type ServiceResult<T> = Result<T, ServiceError>;
