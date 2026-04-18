use entity::user::{ActiveModel, Model};
use sea_orm::prelude::Uuid;
use sea_orm::{ActiveModelTrait, ColumnTrait, IntoActiveModel, Set, TryIntoModel};
use sea_orm::{DatabaseConnection, QueryFilter};
use sea_orm::{DbConn, DbErr, EntityTrait};

/// Look up a user by their API key.
pub async fn get_user_by_api_key(token: String, db_conn: &DbConn) -> Result<Option<Model>, DbErr> {
	let user = entity::user::Entity::find()
		.filter(entity::user::Column::ApiKey.eq(token))
		.one(db_conn)
		.await?;

	Ok(user)
}

/// Look up a user by their internal UUID.
pub async fn get_user_by_id(id: Uuid, db_conn: &DbConn) -> Result<Option<Model>, DbErr> {
	let user = entity::user::Entity::find_by_id(id).one(db_conn).await?;

	Ok(user)
}

/// Look up a user by their Discord id.
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

/// Set the permission level of the given user and persist the change.
pub async fn update_user_permission_level(
	user: Model,
	permissions: entity::sea_orm_active_enums::UserPermissionsEnum,
	db_conn: &DbConn,
) -> Result<Model, DbErr> {
	let mut active_model: entity::user::ActiveModel = user.into_active_model();
	active_model.permissions = Set(permissions);

	let updated_user = active_model.update(db_conn).await?;

	updated_user.try_into_model()
}

/// Insert a new user row and return the persisted model.
pub async fn insert_user(user: ActiveModel, db_conn: &DatabaseConnection) -> Result<Model, DbErr> {
	entity::user::Entity::insert(user)
		.exec_with_returning(db_conn)
		.await
}
