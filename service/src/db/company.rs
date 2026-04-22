use crate::db::abstraction::ColumnEqIgnoreCaseTrait;
use crate::db::unmatched_entities_with_limit;
use entity::company::ActiveModel;
use entity::prelude::Company;
use entity::sea_orm_active_enums::{MatchTypeEnum, MetadataProviderEnum};
use entity::{company, signature_metadata_mapping};
use sea_orm::ActiveValue::Set;
use sea_orm::prelude::Uuid;
use sea_orm::{
	ActiveModelTrait, ColumnTrait, DbConn, DbErr, EntityTrait, ModelTrait, QueryFilter, QueryOrder,
	QuerySelect, TryIntoModel,
};

/// Find a company by exact (case-sensitive) name.
pub async fn find_company_by_name(
	name: &str,
	conn: &DbConn,
) -> Result<Option<company::Model>, DbErr> {
	let company = company::Entity::find()
		.filter(company::Column::Name.eq(name))
		.one(conn)
		.await?;

	Ok(company)
}

/// Return the signature metadata mapping for the given provider attached to a company, if any.
pub async fn find_company_related_signature_metadata_mapping(
	model: &company::Model,
	provider: MetadataProviderEnum,
	conn: &DbConn,
) -> Result<Option<signature_metadata_mapping::Model>, DbErr> {
	model
		.find_related(signature_metadata_mapping::Entity)
		.filter(signature_metadata_mapping::Column::Provider.eq(provider))
		.one(conn)
		.await
}

/// Load a company by id together with all of its signature metadata mappings.
pub async fn get_by_id_and_join_signature_metadata_mappings(
	id: Uuid,
	conn: &DbConn,
) -> Result<Option<(company::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	let company = company::Entity::find()
		.filter(company::Column::Id.eq(id))
		.one(conn)
		.await?;

	if let Some(company) = company {
		let mappings = company
			.find_related(signature_metadata_mapping::Entity)
			.all(conn)
			.await?;

		Ok(Some((company, mappings)))
	} else {
		Ok(None)
	}
}

/// Return every company paired with its signature metadata mappings.
pub async fn find_all_and_join_signature_metadata_mapping(
	conn: &DbConn,
) -> Result<Vec<(company::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	let companies_with_mappings = company::Entity::find()
		.find_with_related(signature_metadata_mapping::Entity)
		.all(conn)
		.await?;

	Ok(companies_with_mappings)
}

/// Find a company by case-insensitive name, creating one with that name if it does not exist.
pub async fn create_or_find_company_by_name(
	name: &str,
	conn: &DbConn,
) -> Result<company::Model, DbErr> {
	let company = Company::find()
		.filter(company::Column::Name.eq_ignore_case(name))
		.one(conn)
		.await?;

	if let Some(company) = company {
		Ok(company)
	} else {
		let mut company = ActiveModel {
			name: Set(name.to_string()),
			..Default::default()
		};

		company = company.save(conn).await?;

		Ok(company.try_into_model()?)
	}
}

unmatched_entities_with_limit! {
	/// Return up to `limit` companies that have no metadata mapping yet for the given provider
	/// (or one with match_type None). Returns `Ok(None)` when there is nothing left to process.
	get_unmatched_companies_with_limit,
	Company,
	company::Model,
	company::Column::Id,
	signature_metadata_mapping::Column::CompanyId
}
