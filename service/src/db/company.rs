use entity::company::ActiveModel;
use entity::prelude::Company;
use entity::sea_orm_active_enums::MatchTypeEnum;
use entity::{company, signature_metadata_mapping};
use sea_orm::ActiveValue::Set;
use sea_orm::{
	ActiveModelTrait, ColumnTrait, DbConn, DbErr, EntityTrait, Paginator, PaginatorTrait,
	QueryFilter, QueryOrder, SelectModel, TryIntoModel,
};

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

pub fn get_companies_unmatched_paginator(
	page_size: u64,
	db_conn: &DbConn,
) -> Paginator<DbConn, SelectModel<company::Model>> {
	Company::find()
		.left_join(signature_metadata_mapping::Entity)
		.filter(
			signature_metadata_mapping::Column::Id
				.is_null()
				.or(signature_metadata_mapping::Column::MatchType.eq(MatchTypeEnum::None)),
		)
		.order_by_desc(company::Column::CreatedAt)
		.paginate(db_conn, page_size)
}
