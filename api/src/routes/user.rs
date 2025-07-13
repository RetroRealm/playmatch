use crate::error;
use crate::model::user::GetUserQuery;
use actix_web::web::{Data, Path, Query};
use actix_web::{HttpResponse, Responder, get};
use sea_orm::DatabaseConnection;
use service::db::user;
use service::model::user::User;
use uuid::Uuid;

/// This Endpoint requires Credentials of a User with at least Automation level.
/// Find a User by their Discord ID.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "User",
	params(GetUserQuery),
	responses(
		(status = 200, description = "Successfully found user", body = User),
		(status = 404, description = "Could not find User with the provided discord id")
	)
)]
#[get("/user")]
pub async fn get_user_by_discord_id(
	query: Query<GetUserQuery>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let user = user::get_user_by_discord_id(query.discord_id, db_conn.get_ref()).await?;

	if let Some(user) = user {
		Ok(HttpResponse::Ok().json(User::from(user)))
	} else {
		Err(error::Error::UserNotFound)
	}
}

/// This Endpoint requires Credentials of a User with at least Automation level.
/// Find a User by their ID.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "User",
	responses(
		(status = 200, description = "Successfully found user", body = User),
		(status = 404, description = "Could not find User with the provided id")
	)
)]
#[get("/user/{id}")]
pub async fn get_user(
	id: Path<Uuid>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let user = user::get_user_by_id(id.into_inner(), db_conn.get_ref()).await?;

	if let Some(user) = user {
		Ok(HttpResponse::Ok().json(User::from(user)))
	} else {
		Err(error::Error::UserNotFound)
	}
}
