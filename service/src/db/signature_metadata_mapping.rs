use derive_builder::Builder;
use entity::sea_orm_active_enums::{
	AutomaticMatchReasonEnum, FailedMatchReasonEnum, ManualMatchModeEnum, MatchTypeEnum,
	MetadataProviderEnum,
};
use entity::signature_metadata_mapping;
use sea_orm::prelude::Uuid;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, DbConn, EntityTrait, QueryFilter, TryIntoModel};

#[derive(Debug, Clone, Builder)]
pub struct SignatureMetadataMappingInput {
	pub provider: MetadataProviderEnum,
	#[builder(default)]
	pub provider_id: Option<String>,
	#[builder(default)]
	pub comment: Option<String>,
	#[builder(default)]
	pub company_id: Option<Uuid>,
	#[builder(default)]
	pub game_id: Option<Uuid>,
	#[builder(default)]
	pub platform_id: Option<Uuid>,
	pub match_type: MatchTypeEnum,
	#[builder(default)]
	pub manual_match_type: Option<ManualMatchModeEnum>,
	#[builder(default)]
	pub failed_match_reason: Option<FailedMatchReasonEnum>,
	#[builder(default)]
	pub automatic_match_reason: Option<AutomaticMatchReasonEnum>,
}

pub async fn create_or_update_signature_metadata_mapping(
	input: SignatureMetadataMappingInput,
	db_conn: &DbConn,
) -> anyhow::Result<signature_metadata_mapping::Model> {
	let signature_metadata_mapping = signature_metadata_mapping::Entity::find()
		.filter(signature_metadata_mapping::Column::PlatformId.eq(input.platform_id))
		.filter(signature_metadata_mapping::Column::GameId.eq(input.game_id))
		.filter(signature_metadata_mapping::Column::CompanyId.eq(input.company_id))
		.filter(signature_metadata_mapping::Column::Provider.eq(input.provider.clone()))
		.one(db_conn)
		.await?;

	if let Some(signature_metadata_mapping) = signature_metadata_mapping {
		Ok(signature_metadata_mapping)
	} else {
		let mut signature_metadata_mapping = signature_metadata_mapping::ActiveModel {
			platform_id: Set(input.platform_id),
			game_id: Set(input.game_id),
			company_id: Set(input.company_id),
			provider: Set(input.provider),
			provider_id: Set(input.provider_id),
			match_type: Set(input.match_type),
			manual_match_type: Set(input.manual_match_type),
			failed_match_reason: Set(input.failed_match_reason),
			comment: Set(input.comment),
			automatic_match_reason: Set(input.automatic_match_reason),
			..Default::default()
		};

		signature_metadata_mapping = signature_metadata_mapping.save(db_conn).await?;

		Ok(signature_metadata_mapping.try_into_model()?)
	}
}
