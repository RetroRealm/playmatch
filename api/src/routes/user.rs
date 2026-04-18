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

/// Find or Create a User by their Discord ID.
/// This Endpoint requires Credentials of a User with at least Automation level.
#[utoipa::path(
	post,
	context_path = "/api",
	tag = "User",
	security(
        ("bearer_auth" = [])
	),
	responses(
		(status = 200, description = "Successfully created or got user", body = User),
		(status = 401, description = "Unauthorized, you need to be logged in to get a user by discord id"),
		(status = 403, description = "Forbidden, you do not have permission to get a user by discord id")
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

/// Find a User by their Discord ID.
/// This Endpoint requires Credentials of a User with at least Automation level.
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
		(status = 401, description = "Unauthorized, you need to be logged in to get a user by discord id"),
		(status = 403, description = "Forbidden, you do not have permission to get a user by discord id"),
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

/// Find a User by their ID.
/// This Endpoint requires Credentials of a User with at least Automation level.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "User",
	security(
        ("bearer_auth" = [])
	),
	responses(
		(status = 200, description = "Successfully found user", body = User),
		(status = 401, description = "Unauthorized, you need to be logged in to get a user by id"),
		(status = 403, description = "Forbidden, you do not have permission to get a user by id"),
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

/// Update a user's Permission Level.
/// This Endpoint requires Credentials of a User with at least Automation level.
#[utoipa::path(
	patch,
	context_path = "/api",
	tag = "User",
	security(
        ("bearer_auth" = [])
	),
	responses(
		(status = 200, description = "Successfully updated user permission level", body = User),
		(status = 401, description = "Unauthorized, you need to be logged in to update a user permission level"),
		(status = 403, description = "Forbidden, you do not have permission to update a user permission level"),
		(status = 404, description = "Could not find User with the provided id")
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
