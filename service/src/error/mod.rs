use crate::db::signature_metadata_mapping::SignatureMetadataMappingInputBuilderError;

#[derive(thiserror::Error, Debug)]
pub enum ServiceError {
	#[error("The game couldn't be found by the given hashes or file name")]
	GameNotFound,

	#[error(transparent)]
	SignatureMetadataMappingInputBuilderError(#[from] SignatureMetadataMappingInputBuilderError),

	#[error(transparent)]
	UpdatedMatchResultBuilderError(#[from] crate::model::UpdatedMatchResultBuilderError),

	#[error(transparent)]
	DbError(#[from] sea_orm::DbErr),
}

pub type ServiceResult<T> = Result<T, ServiceError>;
