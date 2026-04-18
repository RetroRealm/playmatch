use crate::db::abstraction::ColumnEqIgnoreCaseTrait;
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

/// Return the IGDB signature metadata mapping attached to a company, if any.
pub async fn find_company_related_signature_metadata_mapping(
	model: &company::Model,
	conn: &DbConn,
) -> Result<Option<signature_metadata_mapping::Model>, DbErr> {
	model
		.find_related(signature_metadata_mapping::Entity)
		.filter(signature_metadata_mapping::Column::Provider.eq(MetadataProviderEnum::Igdb))
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

/// Return up to `limit` companies that have no IGDB metadata mapping yet (or one with match_type None).
/// Returns `Ok(None)` when there is nothing left to process.
pub async fn get_unmatched_companies_with_limit(
	limit: u64,
	db_conn: &DbConn,
) -> anyhow::Result<Option<Vec<company::Model>>> {
	let found_companies = Company::find()
		.left_join(signature_metadata_mapping::Entity)
		.filter(
			signature_metadata_mapping::Column::Id
				.is_null()
				.or(signature_metadata_mapping::Column::MatchType.eq(MatchTypeEnum::None)),
		)
		.order_by_asc(company::Column::Id)
		.limit(limit)
		.all(db_conn)
		.await?;

	if found_companies.is_empty() {
		Ok(None)
	} else {
		Ok(Some(found_companies))
	}
}
