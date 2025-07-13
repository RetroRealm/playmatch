use crate::error;
use crate::model::user::{GetUserQuery, UpdateUserPermissionsQuery};
use crate::routes::handle_auth_and_permissions;
use actix_web::web::{Data, Path, Query};
use actix_web::{HttpRequest, HttpResponse, Responder, get, patch};
use entity::sea_orm_active_enums::UserPermissionsEnum;
use sea_orm::DatabaseConnection;
use service::user;
use uuid::Uuid;

/// This Endpoint requires Credentials of a User with at least Automation level.
/// Find a User by their Discord ID.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "User",
	params(GetUserQuery),
	security(
        ("bearer_auth" = [])
	),
	responses(
		(status = 200, description = "Successfully found user", body = User),
		(status = 404, description = "Could not find User with the provided discord id")
	)
)]
#[get("/user")]
pub async fn get_user_by_discord_id(
	query: Query<GetUserQuery>,
	db_conn: Data<DatabaseConnection>,
	req: HttpRequest,
) -> error::Result<impl Responder> {
	handle_auth_and_permissions(&UserPermissionsEnum::Automation, req, db_conn.clone()).await?;

	Ok(HttpResponse::Ok()
		.json(user::get_user_by_discord_id(query.discord_id, db_conn.get_ref()).await?))
}

/// This Endpoint requires Credentials of a User with at least Automation level.
/// Find a User by their ID.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "User",
	security(
        ("bearer_auth" = [])
	),
	responses(
		(status = 200, description = "Successfully found user", body = User),
		(status = 404, description = "Could not find User with the provided id")
	)
)]
#[get("/user/{id}")]
pub async fn get_user(
	id: Path<Uuid>,
	db_conn: Data<DatabaseConnection>,
	req: HttpRequest,
) -> error::Result<impl Responder> {
	handle_auth_and_permissions(&UserPermissionsEnum::Automation, req, db_conn.clone()).await?;

	Ok(HttpResponse::Ok().json(user::get_user_by_id(id.into_inner(), db_conn.get_ref()).await?))
}

/// This Endpoint requires Credentials of a User with at least Automation level.
/// Find a User by their ID.
#[utoipa::path(
	patch,
	context_path = "/api",
	tag = "User",
	security(
        ("bearer_auth" = [])
	),
	params(UpdateUserPermissionsQuery),
	responses(
		(status = 200, description = "Successfully update user permission level", body = User),
		(status = 404, description = "Could not find User with the provided id")
	)
)]
#[patch("/user/{id}/permission")]
pub async fn update_user_permission_level(
	id: Path<Uuid>,
	query: Query<UpdateUserPermissionsQuery>,
	db_conn: Data<DatabaseConnection>,
	req: HttpRequest,
) -> error::Result<impl Responder> {
	handle_auth_and_permissions(&UserPermissionsEnum::Automation, req, db_conn.clone()).await?;

	Ok(HttpResponse::Ok().json(
		user::update_permission_level(id.into_inner(), query.into_inner().new_permission, &db_conn)
			.await?,
	))
}
