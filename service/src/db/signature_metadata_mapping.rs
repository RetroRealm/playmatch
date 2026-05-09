use crate::db::abstraction::ColumnNullTrait;
use chrono::Utc;
use derive_builder::Builder;
use entity::sea_orm_active_enums::{
	AutomaticMatchReasonEnum, FailedMatchReasonEnum, ManualMatchModeEnum, MatchTypeEnum,
	MetadataProviderEnum,
};
use entity::signature_metadata_mapping;
use entity::signature_metadata_mapping::Model;
use sea_orm::ActiveValue::Set;
use sea_orm::prelude::Uuid;
use sea_orm::sea_query::Expr;
use sea_orm::{
	ActiveModelTrait, ColumnTrait, DbConn, DbErr, EntityTrait, IntoActiveModel, QueryFilter,
	TryIntoModel,
};

/// Builder input for [`create_or_update_signature_metadata_mapping`]. Exactly one of
/// `company_id`, `game_id`, `platform_id` should be set.
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
	#[builder(default)]
	pub manually_matched_by: Option<Uuid>,
	#[builder(default)]
	pub matched_name: Option<String>,
	#[builder(default)]
	pub matched_year: Option<i16>,
}

/// Stamp `cross_match_last_tried_at = now()` on every Failed-NoDirectMatch
/// row for `provider` whose `game_id` is in `game_ids`. Called by the
/// cross-pass orchestrator after each chunk attempt so the same games do
/// not return on the next page-fetch loop iteration. Bulk update: one
/// statement per chunk, not one per row.
pub async fn bulk_stamp_cross_match_attempt(
	provider: MetadataProviderEnum,
	game_ids: &[Uuid],
	conn: &DbConn,
) -> Result<(), DbErr> {
	if game_ids.is_empty() {
		return Ok(());
	}
	signature_metadata_mapping::Entity::update_many()
		.col_expr(
			signature_metadata_mapping::Column::CrossMatchLastTriedAt,
			Expr::value(Utc::now().fixed_offset()),
		)
		.filter(signature_metadata_mapping::Column::Provider.eq(provider))
		.filter(signature_metadata_mapping::Column::MatchType.eq(MatchTypeEnum::Failed))
		.filter(
			signature_metadata_mapping::Column::FailedMatchReason
				.eq(FailedMatchReasonEnum::NoDirectMatch),
		)
		.filter(signature_metadata_mapping::Column::GameId.is_in(game_ids.iter().copied()))
		.exec(conn)
		.await?;
	Ok(())
}

/// Look up sibling provider mappings for a game, returning each sibling
/// provider's canonical title as recorded in `matched_name`. Used by the
/// cross-provider name pass: the caller filters its own provider out via
/// `own_provider` so the result only contains usable foreign-provider
/// queries. Mappings without a `matched_name` are skipped entirely.
pub async fn find_sibling_matched_names(
	game_id: Uuid,
	own_provider: MetadataProviderEnum,
	conn: &DbConn,
) -> Result<Vec<(MetadataProviderEnum, String)>, DbErr> {
	let rows = signature_metadata_mapping::Entity::find()
		.filter(signature_metadata_mapping::Column::GameId.eq(game_id))
		.filter(signature_metadata_mapping::Column::Provider.ne(own_provider))
		.filter(
			signature_metadata_mapping::Column::MatchType
				.eq(MatchTypeEnum::Automatic)
				.or(signature_metadata_mapping::Column::MatchType.eq(MatchTypeEnum::Manual)),
		)
		.filter(signature_metadata_mapping::Column::MatchedName.is_not_null())
		.all(conn)
		.await?;
	Ok(rows
		.into_iter()
		.filter_map(|m| m.matched_name.map(|n| (m.provider, n)))
		.collect())
}

/// Look up the mapping that targets the given platform, game, company and provider tuple.
pub async fn find_signature_metadata_mapping_by_platform_game_company_and_provider(
	platform_id: Option<Uuid>,
	game_id: Option<Uuid>,
	company_id: Option<Uuid>,
	provider: MetadataProviderEnum,
	db_conn: &DbConn,
) -> Result<Option<Model>, DbErr> {
	signature_metadata_mapping::Entity::find()
		.filter(signature_metadata_mapping::Column::PlatformId.eq_null(platform_id))
		.filter(signature_metadata_mapping::Column::GameId.eq_null(game_id))
		.filter(signature_metadata_mapping::Column::CompanyId.eq_null(company_id))
		.filter(signature_metadata_mapping::Column::Provider.eq(provider))
		.one(db_conn)
		.await
}

/// Upsert a signature metadata mapping. Looks up the existing row by its identifying tuple,
/// updates it in place if found, or inserts a new row otherwise. Touches `updated_at` on every call.
pub async fn create_or_update_signature_metadata_mapping(
	input: SignatureMetadataMappingInput,
	db_conn: &DbConn,
) -> Result<Model, DbErr> {
	let signature_metadata_mapping =
		find_signature_metadata_mapping_by_platform_game_company_and_provider(
			input.platform_id,
			input.game_id,
			input.company_id,
			input.provider,
			db_conn,
		)
		.await?;

	let mut active_model = if let Some(signature_metadata_mapping) = signature_metadata_mapping {
		signature_metadata_mapping.into_active_model()
	} else {
		signature_metadata_mapping::ActiveModel {
			..Default::default()
		}
	};

	active_model.platform_id = Set(input.platform_id);
	active_model.game_id = Set(input.game_id);
	active_model.company_id = Set(input.company_id);
	active_model.provider = Set(input.provider);
	active_model.provider_id = Set(input.provider_id);
	active_model.match_type = Set(input.match_type);
	active_model.manual_match_type = Set(input.manual_match_type);
	active_model.failed_match_reason = Set(input.failed_match_reason);
	active_model.comment = Set(input.comment);
	active_model.automatic_match_reason = Set(input.automatic_match_reason);
	active_model.manually_matched_by = Set(input.manually_matched_by);
	if input.matched_name.is_some() {
		active_model.matched_name = Set(input.matched_name);
	}
	if input.matched_year.is_some() {
		active_model.matched_year = Set(input.matched_year);
	}
	active_model.updated_at = Set(Utc::now().fixed_offset());

	active_model = active_model.save(db_conn).await?;

	active_model.try_into_model()
}
