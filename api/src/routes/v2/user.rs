use crate::error;
use crate::routes::handle_auth_and_permissions;
use actix_web::web::{Data, Json, Path, Query};
use actix_web::{HttpRequest, HttpResponse, Responder, get, patch, post};
use entity::sea_orm_active_enums::UserPermissionsEnum;
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use service::entities::user;
use service::model::user::{
	CreateOrGetUserRequest, CreateOrGetUserRequestV2, UpdateUserPermissionsRequestV2,
};
use utoipa::IntoParams;
use uuid::Uuid;

/// Query parameters for the v2 get-user-by-Discord-id lookup. camelCase on the
/// wire (`discordId`) so it matches the rest of the v2 surface.
#[derive(Debug, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct GetUserQueryV2 {
	pub discord_id: i64,
}

/// Returns a user by Discord id.
///
/// Requires credentials of at least Automation level.
#[utoipa::path(
	get,
	tag = "User",
	params(GetUserQueryV2),
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
pub async fn get_user_by_discord_id_v2(
	query: Query<GetUserQueryV2>,
	db_conn: Data<DatabaseConnection>,
	req: HttpRequest,
) -> error::Result<impl Responder> {
	handle_auth_and_permissions(&UserPermissionsEnum::Automation, req, db_conn.clone()).await?;

	Ok(HttpResponse::Ok()
		.json(user::get_user_by_discord_id(query.discord_id, db_conn.get_ref()).await?))
}

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
pub async fn create_or_get_by_discord_id_v2(
	body: Json<CreateOrGetUserRequestV2>,
	db_conn: Data<DatabaseConnection>,
	req: HttpRequest,
) -> error::Result<impl Responder> {
	handle_auth_and_permissions(&UserPermissionsEnum::Automation, req, db_conn.clone()).await?;

	let request = CreateOrGetUserRequest::from(body.into_inner());
	Ok(HttpResponse::Ok()
		.json(user::create_or_get_user_by_discord_id(request, db_conn.get_ref()).await?))
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
pub async fn update_user_permission_level_v2(
	id: Path<Uuid>,
	body: Json<UpdateUserPermissionsRequestV2>,
	db_conn: Data<DatabaseConnection>,
	req: HttpRequest,
) -> error::Result<impl Responder> {
	handle_auth_and_permissions(&UserPermissionsEnum::Automation, req, db_conn.clone()).await?;

	Ok(HttpResponse::Ok().json(
		user::update_permission_level(id.into_inner(), body.into_inner().new_permission, &db_conn)
			.await?,
	))
}
