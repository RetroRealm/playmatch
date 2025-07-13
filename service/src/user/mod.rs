use crate::db::user;
use crate::error::ServiceError;
use crate::model::user::{User, UserPermissions};
use sea_orm::DatabaseConnection;
use sea_orm::prelude::Uuid;

pub async fn get_user_by_id(
	id: Uuid,
	db_conn: &DatabaseConnection,
) -> crate::error::ServiceResult<User> {
	let user = user::get_user_by_id(id, db_conn).await?;

	if let Some(user) = user {
		Ok(user.into())
	} else {
		Err(ServiceError::UserNotFound)
	}
}

pub async fn get_user_by_discord_id(
	discord_id: i64,
	db_conn: &DatabaseConnection,
) -> crate::error::ServiceResult<User> {
	let user = user::get_user_by_discord_id(discord_id, db_conn).await?;

	if let Some(user) = user {
		Ok(user.into())
	} else {
		Err(ServiceError::UserNotFound)
	}
}

pub async fn update_permission_level(
	id: Uuid,
	permission_level: UserPermissions,
	db_conn: &DatabaseConnection,
) -> crate::error::ServiceResult<User> {
	let user = user::get_user_by_id(id, db_conn).await?;

	let user = match user {
		None => Err(ServiceError::UserNotFound)?,
		Some(user) => user,
	};

	let updated_user =
		user::update_user_permission_level(user, permission_level.into(), db_conn).await?;

	Ok(updated_user.into())
}
