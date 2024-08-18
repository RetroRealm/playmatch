use entity::sea_orm_active_enums::{
	FailedMatchReasonEnum, ManualMatchModeEnum, MatchTypeEnum, MetadataProviderEnum,
};
use entity::signature_metadata_mapping;
use sea_orm::prelude::Uuid;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, DbConn, EntityTrait, QueryFilter, TryIntoModel};

#[derive(Debug, Clone)]
pub struct SignatureMetadataMappingInput {
	pub provider_name: MetadataProviderEnum,
	pub provider_id: Option<String>,
	pub comment: Option<String>,
	pub company_id: Option<Uuid>,
	pub game_id: Option<Uuid>,
	pub platform_id: Option<Uuid>,
	pub match_type: MatchTypeEnum,
	pub manual_match_type: Option<ManualMatchModeEnum>,
	pub failed_match_reason: Option<FailedMatchReasonEnum>,
}

pub async fn create_or_update_signature_metadata_mapping(
	input: SignatureMetadataMappingInput,
	db_conn: &DbConn,
) -> anyhow::Result<signature_metadata_mapping::Model> {
	let signature_metadata_mapping = signature_metadata_mapping::Entity::find()
		.filter(signature_metadata_mapping::Column::PlatformId.eq(input.platform_id))
		.filter(signature_metadata_mapping::Column::GameId.eq(input.game_id))
		.filter(signature_metadata_mapping::Column::CompanyId.eq(input.company_id))
		.filter(signature_metadata_mapping::Column::ProviderName.eq(input.provider_name.clone()))
		.one(db_conn)
		.await?;

	if let Some(signature_metadata_mapping) = signature_metadata_mapping {
		Ok(signature_metadata_mapping)
	} else {
		let mut signature_metadata_mapping = signature_metadata_mapping::ActiveModel {
			platform_id: Set(input.platform_id),
			game_id: Set(input.game_id),
			company_id: Set(input.company_id),
			provider_name: Set(input.provider_name),
			provider_id: Set(input.provider_id),
			match_type: Set(input.match_type),
			manual_match_type: Set(input.manual_match_type),
			failed_match_reason: Set(input.failed_match_reason),
			comment: Set(input.comment),
			..Default::default()
		};

		signature_metadata_mapping = signature_metadata_mapping.save(db_conn).await?;

		Ok(signature_metadata_mapping.try_into_model()?)
	}
}
