use entity::user::Model;
use sea_orm::ColumnTrait;
use sea_orm::QueryFilter;
use sea_orm::prelude::Uuid;
use sea_orm::{DbConn, DbErr, EntityTrait};

pub async fn get_user_by_api_key(token: String, db_conn: &DbConn) -> Result<Option<Model>, DbErr> {
	let user = entity::user::Entity::find()
		.filter(entity::user::Column::ApiKey.eq(token))
		.one(db_conn)
		.await?;

	Ok(user)
}

pub async fn get_user_by_id(id: Uuid, db_conn: &DbConn) -> Result<Option<Model>, DbErr> {
	let user = entity::user::Entity::find_by_id(id).one(db_conn).await?;

	Ok(user)
}

pub async fn get_user_by_discord_id(
	discord_id: i64,
	db_conn: &DbConn,
) -> Result<Option<Model>, DbErr> {
	let user = entity::user::Entity::find()
		.filter(entity::user::Column::DiscordId.eq(discord_id))
		.one(db_conn)
		.await?;

	Ok(user)
}
