use entity::platform::ActiveModel;
use entity::prelude::Platform;
use entity::sea_orm_active_enums::MatchTypeEnum;
use entity::{platform, signature_metadata_mapping};
use sea_orm::prelude::Uuid;
use sea_orm::ActiveValue::Set;
use sea_orm::{
	ActiveModelTrait, ColumnTrait, DbConn, DbErr, EntityTrait, Paginator, PaginatorTrait,
	QueryFilter, SelectModel, TryIntoModel,
};

pub async fn create_or_find_platform_by_name(
	name: &str,
	company_id: Option<Uuid>,
	conn: &DbConn,
) -> Result<platform::Model, DbErr> {
	let platform = Platform::find()
		.filter(platform::Column::Name.eq(name))
		.one(conn)
		.await?;

	if let Some(platform) = platform {
		Ok(platform)
	} else {
		let mut platform = ActiveModel {
			name: Set(name.to_string()),
			company_id: Set(company_id),
			..Default::default()
		};

		platform = platform.save(conn).await?;

		Ok(platform.try_into_model()?)
	}
}

pub fn get_platforms_unmatched_paginator(
	page_size: u64,
	conn: &DbConn,
) -> Paginator<DbConn, SelectModel<platform::Model>> {
	Platform::find()
		.left_join(signature_metadata_mapping::Entity)
		.filter(
			signature_metadata_mapping::Column::Id
				.is_null()
				.or(signature_metadata_mapping::Column::MatchType.eq(MatchTypeEnum::None)),
		)
		.paginate(conn, page_size)
}
