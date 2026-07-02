use crate::error;
use crate::error::Error::{InvalidAuth, InvalidAuthPermission};
use actix_web::web::Data;
use actix_web::{HttpRequest, HttpResponse};
use entity::sea_orm_active_enums::UserPermissionsEnum;
use sea_orm::DatabaseConnection;
use serde::Serialize;
use service::db::user::get_user_by_api_key;

mod macros;

pub mod company;
pub mod game;
pub mod health;
pub mod identify;
pub mod igdb;
pub mod launchbox;
pub mod r#match;
pub mod mobygames;
pub mod openvgdb;
pub mod platform;
pub mod retroachievements;
pub mod screenscraper;
pub mod sgdb;
pub mod signature_group;
pub mod suggestion;
pub mod user;
pub mod v2;
pub mod versions;

async fn handle_auth_and_permissions(
	required_user_perms: &UserPermissionsEnum,
	req: HttpRequest,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<entity::user::Model> {
	// One message for every auth failure path; distinct bodies would leak whether a token exists.
	const INVALID_AUTH_BODY: &str = "invalid or missing credentials";
	let started = std::time::Instant::now();
	let finish = |outcome: &'static str| {
		service::metrics::record_auth_attempt(outcome);
		service::metrics::observe_auth_latency(outcome, started.elapsed().as_secs_f64());
	};

	let token = match req
		.headers()
		.get("Authorization")
		.and_then(|h| h.to_str().ok())
		.and_then(|h| h.strip_prefix("Bearer "))
		.filter(|t| !t.is_empty())
	{
		Some(t) => t,
		None => {
			finish("missing_header");
			return Err(InvalidAuth(INVALID_AUTH_BODY.to_string()));
		}
	};

	let user = match get_user_by_api_key(token.to_string(), db_conn.get_ref()).await? {
		Some(u) => u,
		None => {
			finish("unknown_token");
			return Err(InvalidAuth(INVALID_AUTH_BODY.to_string()));
		}
	};

	let permitted = match required_user_perms {
		UserPermissionsEnum::User => true,
		UserPermissionsEnum::Trusted => [
			UserPermissionsEnum::Trusted,
			UserPermissionsEnum::Automation,
			UserPermissionsEnum::Admin,
		]
		.contains(&user.permissions),
		UserPermissionsEnum::Automation => {
			user.permissions == UserPermissionsEnum::Automation
				|| user.permissions == UserPermissionsEnum::Admin
		}
		UserPermissionsEnum::Admin => user.permissions == UserPermissionsEnum::Admin,
	};

	if permitted {
		finish("success");
		Ok(user)
	} else {
		finish("permission_denied");
		Err(InvalidAuthPermission)
	}
}

pub(crate) fn ok_or_not_found<T: Serialize>(opt: Option<T>) -> HttpResponse {
	match opt {
		Some(v) => HttpResponse::Ok().json(v),
		None => HttpResponse::NotFound().finish(),
	}
}
