use crate::db::signature_metadata_mapping::SignatureMetadataMappingInputBuilderError;

#[derive(thiserror::Error, Debug)]
pub enum ServiceError {
	#[error("The game couldn't be found by the given hashes or file name")]
	GameNotFound,

	#[error("The platform couldn't be found by the given name")]
	PlatformNotFound,

	#[error("The company couldn't be found by the given name")]
	CompanyNotFound,

	#[error("The user couldn't be found by the given ID")]
	UserNotFound,

	#[error(
		"A suggestion for this Game/Platform/Company with the same provider and provider ID already exists"
	)]
	SuggestionAlreadyExists,

	#[error("The suggestion with the given ID couldn't be found")]
	SuggestionNotFound,

	#[error(transparent)]
	SignatureMetadataMappingInputBuilderError(#[from] SignatureMetadataMappingInputBuilderError),

	#[error(transparent)]
	UpdatedMatchResultBuilderError(#[from] crate::model::UpdatedMatchResultBuilderError),

	#[error(transparent)]
	DbError(#[from] sea_orm::DbErr),
}

pub type ServiceResult<T> = Result<T, ServiceError>;
