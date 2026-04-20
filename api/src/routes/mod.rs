use crate::error;
use crate::error::Error::{InvalidAuth, InvalidAuthPermission};
use actix_web::web::Data;
use actix_web::{HttpRequest, HttpResponse};
use entity::sea_orm_active_enums::UserPermissionsEnum;
use sea_orm::DatabaseConnection;
use serde::Serialize;
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
	// One message for every auth failure path; distinct bodies would leak whether a token exists.
	const INVALID_AUTH_BODY: &str = "Invalid or missing credentials.";

	let token = req
		.headers()
		.get("Authorization")
		.and_then(|h| h.to_str().ok())
		.and_then(|h| h.strip_prefix("Bearer "))
		.filter(|t| !t.is_empty())
		.ok_or_else(|| InvalidAuth(INVALID_AUTH_BODY.to_string()))?;

	let user = get_user_by_api_key(token.to_string(), db_conn.get_ref())
		.await?
		.ok_or_else(|| InvalidAuth(INVALID_AUTH_BODY.to_string()))?;

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
		UserPermissionsEnum::Admin if user.permissions == UserPermissionsEnum::Admin => Ok(user),
		_ => Err(InvalidAuthPermission),
	}
}

pub(crate) fn ok_or_not_found<T: Serialize>(opt: Option<T>) -> HttpResponse {
	match opt {
		Some(v) => HttpResponse::Ok().json(v),
		None => HttpResponse::NotFound().finish(),
	}
}
