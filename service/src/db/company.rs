use crate::db::abstraction::ColumnEqIgnoreCaseTrait;
use crate::db::pagination::{KeysetPage, fetch_keyset_page};
use crate::db::unmatched_entities_with_limit;
use entity::company::ActiveModel;
use entity::prelude::Company;
use entity::sea_orm_active_enums::{MatchTypeEnum, MetadataProviderEnum};
use entity::{company, signature_metadata_mapping};
use sea_orm::ActiveValue::Set;
use sea_orm::prelude::Uuid;
use sea_orm::sea_query::Expr;
use sea_orm::sea_query::extension::postgres::PgExpr;
use sea_orm::{
	ActiveModelTrait, ColumnTrait, DbConn, DbErr, EntityTrait, LoaderTrait, ModelTrait,
	QueryFilter, QueryOrder, QuerySelect, TryIntoModel,
};
use std::collections::HashMap;

/// Looks up a company by exact, case-sensitive name.
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

/// Load every company in `ids` in one `is_in` query, keyed by id. Bulk variant
/// used to hydrate a page of dat files without a query per row.
pub async fn find_companies_by_ids(
	ids: &[Uuid],
	conn: &DbConn,
) -> Result<HashMap<Uuid, company::Model>, DbErr> {
	if ids.is_empty() {
		return Ok(HashMap::new());
	}
	let rows = company::Entity::find()
		.filter(company::Column::Id.is_in(ids.iter().copied()))
		.all(conn)
		.await?;
	Ok(rows.into_iter().map(|row| (row.id, row)).collect())
}

pub async fn find_all_and_join_signature_metadata_mapping(
	conn: &DbConn,
) -> Result<Vec<(company::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	let companies_with_mappings = company::Entity::find()
		.find_with_related(signature_metadata_mapping::Entity)
		.all(conn)
		.await?;

	Ok(companies_with_mappings)
}

pub async fn find_companies_page_and_join_signature_metadata_mapping(
	after: Option<(String, Uuid)>,
	limit: Option<u64>,
	conn: &DbConn,
) -> Result<KeysetPage<(company::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	let mut cursor =
		company::Entity::find().cursor_by((company::Column::Name, company::Column::Id));
	if let Some((name, id)) = after {
		cursor.after((name, id));
	}
	let page = fetch_keyset_page(&mut cursor, limit, conn).await?;

	let mappings = page
		.rows
		.load_many(signature_metadata_mapping::Entity, conn)
		.await?;

	let rows = page.rows.into_iter().zip(mappings).collect();
	Ok(KeysetPage {
		rows,
		has_more: page.has_more,
	})
}

/// One keyset page of companies whose name matches `query` as a case-insensitive
/// substring, ordered by `(name, id)` and joined to their metadata mappings like
/// [`find_companies_page_and_join_signature_metadata_mapping`]. The match is an
/// ILIKE against a `%query%` pattern; `after` is the last row of the previous page.
pub async fn search_companies_by_name_page(
	query: &str,
	after: Option<(String, Uuid)>,
	limit: Option<u64>,
	conn: &DbConn,
) -> Result<KeysetPage<(company::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	let pattern = format!("%{query}%");

	let mut cursor = company::Entity::find()
		.filter(Expr::col((company::Entity, company::Column::Name)).ilike(pattern))
		.cursor_by((company::Column::Name, company::Column::Id));
	if let Some((name, id)) = after {
		cursor.after((name, id));
	}
	let page = fetch_keyset_page(&mut cursor, limit, conn).await?;

	let mappings = page
		.rows
		.load_many(signature_metadata_mapping::Entity, conn)
		.await?;

	let rows = page.rows.into_iter().zip(mappings).collect();
	Ok(KeysetPage {
		rows,
		has_more: page.has_more,
	})
}

/// Matches a company by case-insensitive name, creating one when none exists.
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
