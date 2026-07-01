use crate::error;
use crate::model::user::GetUserQuery;
use crate::routes::handle_auth_and_permissions;
use actix_web::web::{Data, Json, Path, Query};
use actix_web::{HttpRequest, HttpResponse, Responder, get, patch, post};
use entity::sea_orm_active_enums::UserPermissionsEnum;
use sea_orm::DatabaseConnection;
use service::entities::user;
use service::model::user::{CreateOrGetUserRequest, UpdateUserPermissionsRequest};
use uuid::Uuid;

/// Returns a user by Discord id, creating one if none exists.
///
/// Looks up the user by Discord id and creates a new user when no match is found. Requires credentials of at least Automation level.
#[utoipa::path(
	post,
	tag = "User",
	security(
        ("bearer_auth" = [])
	),
	responses(
		(status = 200, description = "The matching or newly created user", body = User),
		(status = 401, description = "Missing or invalid credentials"),
		(status = 403, description = "Credentials below Automation level")
	)
)]
#[post("/user/by-discord-id")]
pub async fn create_or_get_by_discord_id(
	body: Json<CreateOrGetUserRequest>,
	db_conn: Data<DatabaseConnection>,
	req: HttpRequest,
) -> error::Result<impl Responder> {
	handle_auth_and_permissions(&UserPermissionsEnum::Automation, req, db_conn.clone()).await?;

	Ok(HttpResponse::Ok()
		.json(user::create_or_get_user_by_discord_id(body.into_inner(), db_conn.get_ref()).await?))
}

/// Returns a user by Discord id.
///
/// Requires credentials of at least Automation level.
#[utoipa::path(
	get,
	tag = "User",
	params(GetUserQuery),
	security(
        ("bearer_auth" = [])
	),
	responses(
		(status = 200, description = "The matching user", body = User),
		(status = 401, description = "Missing or invalid credentials"),
		(status = 403, description = "Credentials below Automation level"),
		(status = 404, description = "User not found")
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

/// Returns a user by id.
///
/// Requires credentials of at least Automation level.
#[utoipa::path(
	get,
	tag = "User",
	security(
        ("bearer_auth" = [])
	),
	responses(
		(status = 200, description = "The matching user", body = User),
		(status = 401, description = "Missing or invalid credentials"),
		(status = 403, description = "Credentials below Automation level"),
		(status = 404, description = "User not found")
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

/// Updates a user's permission level.
///
/// Sets the permission level of the user to the value in the request body. Requires credentials of at least Automation level.
#[utoipa::path(
	patch,
	tag = "User",
	security(
        ("bearer_auth" = [])
	),
	responses(
		(status = 200, description = "The user with its updated permission level", body = User),
		(status = 401, description = "Missing or invalid credentials"),
		(status = 403, description = "Credentials below Automation level"),
		(status = 404, description = "User not found")
	)
)]
#[patch("/user/{id}/permission")]
pub async fn update_user_permission_level(
	id: Path<Uuid>,
	body: Json<UpdateUserPermissionsRequest>,
	db_conn: Data<DatabaseConnection>,
	req: HttpRequest,
) -> error::Result<impl Responder> {
	handle_auth_and_permissions(&UserPermissionsEnum::Automation, req, db_conn.clone()).await?;

	Ok(HttpResponse::Ok().json(
		user::update_permission_level(id.into_inner(), body.into_inner().new_permission, &db_conn)
			.await?,
	))
}
