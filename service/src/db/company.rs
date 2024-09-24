use entity::company::ActiveModel;
use entity::prelude::Company;
use entity::sea_orm_active_enums::MatchTypeEnum;
use entity::{company, signature_metadata_mapping};
use sea_orm::ActiveValue::Set;
use sea_orm::{
	ActiveModelTrait, ColumnTrait, DbConn, DbErr, EntityTrait, QueryFilter, QueryOrder,
	QuerySelect, TryIntoModel,
};

pub async fn find_all_and_join_signature_metadata_mapping(
	conn: &DbConn,
) -> Result<Vec<(company::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	let companies_with_mappings = company::Entity::find()
		.find_with_related(signature_metadata_mapping::Entity)
		.all(conn)
		.await?;

	Ok(companies_with_mappings)
}

pub async fn create_or_find_company_by_name(
	name: &str,
	conn: &DbConn,
) -> Result<company::Model, DbErr> {
	let company = Company::find()
		.filter(company::Column::Name.eq(name))
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
