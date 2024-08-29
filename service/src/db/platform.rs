use entity::platform::ActiveModel;
use entity::prelude::Platform;
use entity::sea_orm_active_enums::{MatchTypeEnum, MetadataProviderEnum};
use entity::{dat_file, dat_file_import, game, platform, signature_metadata_mapping};
use sea_orm::prelude::Uuid;
use sea_orm::ActiveValue::Set;
use sea_orm::{
	ActiveModelTrait, ColumnTrait, DbConn, DbErr, EntityTrait, JoinType, ModelTrait, QueryFilter,
	QueryOrder, QuerySelect, RelationTrait, TryIntoModel,
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

pub async fn get_unmatched_platforms_with_limit(
	limit: u64,
	conn: &DbConn,
) -> anyhow::Result<Option<Vec<platform::Model>>> {
	let res = Platform::find()
		.left_join(signature_metadata_mapping::Entity)
		.filter(
			signature_metadata_mapping::Column::Id
				.is_null()
				.or(signature_metadata_mapping::Column::MatchType.eq(MatchTypeEnum::None)),
		)
		.order_by_asc(platform::Column::Id)
		.limit(limit)
		.all(conn)
		.await?;

	if res.is_empty() {
		Ok(None)
	} else {
		Ok(Some(res))
	}
}

pub async fn find_platform_of_game(
	game_id: Uuid,
	conn: &DbConn,
) -> Result<Option<platform::Model>, DbErr> {
	Platform::find()
		.join(JoinType::InnerJoin, platform::Relation::DatFile.def())
		.join(JoinType::InnerJoin, dat_file::Relation::DatFileImport.def())
		.join(JoinType::InnerJoin, dat_file_import::Relation::Game.def())
		.filter(game::Column::Id.eq(game_id))
		.one(conn)
		.await
}

pub async fn find_related_signature_metadata_mapping(
	model: &platform::Model,
	conn: &DbConn,
) -> Result<Option<signature_metadata_mapping::Model>, DbErr> {
	model
		.find_related(signature_metadata_mapping::Entity)
		.filter(signature_metadata_mapping::Column::Provider.eq(MetadataProviderEnum::Igdb))
		.one(conn)
		.await
}
