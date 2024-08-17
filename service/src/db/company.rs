use entity::company;
use entity::company::ActiveModel;
use entity::prelude::Company;
use sea_orm::ActiveValue::Set;
use sea_orm::{
	ActiveModelTrait, ColumnTrait, DbConn, DbErr, EntityTrait, QueryFilter, TryIntoModel,
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
