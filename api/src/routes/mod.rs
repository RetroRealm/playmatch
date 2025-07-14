use crate::error;
use crate::error::Error::{InvalidAuth, InvalidAuthPermission};
use actix_web::HttpRequest;
use actix_web::web::Data;
use entity::sea_orm_active_enums::UserPermissionsEnum;
use sea_orm::DatabaseConnection;
use service::db::user::get_user_by_api_key;

pub mod company;
pub mod game;
pub mod health;
pub mod identify;
pub mod igdb;
pub mod r#match;
pub mod platform;
pub mod suggestion;
pub mod user;

async fn handle_auth_and_permissions(
	required_user_perms: &UserPermissionsEnum,
	req: HttpRequest,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<entity::user::Model> {
	let auth_header = req
		.headers()
		.get("Authorization")
		.and_then(|h| h.to_str().ok());

	if auth_header.is_none() {
		return Err(InvalidAuth("Authorization header is required.".to_string()));
	}

	let auth_header = auth_header.unwrap();

	if !auth_header.starts_with("Bearer ") {
		return Err(InvalidAuth(
			"Authorization header must start with 'Bearer '.".to_string(),
		));
	}

	// Remove "Bearer " prefix
	let token = &auth_header[7..];

	if token.is_empty() {
		return Err(InvalidAuth("Token is required.".to_string()));
	}

	let user = get_user_by_api_key(token.to_string(), db_conn.get_ref()).await?;

	if let Some(user) = user {
		match required_user_perms {
			UserPermissionsEnum::User => Ok(user),
			UserPermissionsEnum::Trusted
				if [
					UserPermissionsEnum::Trusted,
					UserPermissionsEnum::Automation,
					UserPermissionsEnum::Admin,
				]
				.contains(&user.permissions) =>
			{
				Ok(user)
			}
			UserPermissionsEnum::Automation
				if user.permissions == UserPermissionsEnum::Automation
					|| user.permissions == UserPermissionsEnum::Admin =>
			{
				Ok(user)
			}
			UserPermissionsEnum::Admin if user.permissions == UserPermissionsEnum::Admin => {
				Ok(user)
			}
			_ => Err(InvalidAuthPermission),
		}
	} else {
		Err(InvalidAuth("Invalid API token.".to_string()))
	}
}
