use crate::error;
use crate::routes::handle_auth_and_permissions;
use actix_web::web::{Data, Json, Path};
use actix_web::{HttpRequest, HttpResponse, Responder, delete, get, post};
use entity::sea_orm_active_enums::UserPermissionsEnum;
use sea_orm::DatabaseConnection;
use service::matching::suggestions::{
	accept_suggestion, add_company_suggestion, add_game_suggestion, add_platform_suggestion,
	decline_suggestion, get_suggestion, get_suggestions,
};
use service::model::suggestion::{
	CompanyOrPlatformSuggestionRequest, GameSuggestionRequest,
	UpdatedMetadataMatchesFromSuggestionResponse,
};
use uuid::Uuid;

/// Gets all currently pending suggestions.
/// This Endpoint requires Credentials of at least Automation level.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "Suggestion",
	security(
        ("bearer_auth" = [])
	),
	responses(
		(status = 200, description = "Successfully retrieved suggestions", body = Vec<Suggestion>),
		(status = 401, description = "Unauthorized, you need to be logged in to view all suggestion"),
		(status = 403, description = "Forbidden, you do not have permission to view all suggestions"),
	)
)]
#[get("/suggestion")]
pub async fn get_all_suggestions(
	db_conn: Data<DatabaseConnection>,
	req: HttpRequest,
) -> error::Result<impl Responder> {
	handle_auth_and_permissions(&UserPermissionsEnum::Automation, req, db_conn.clone()).await?;

	Ok(HttpResponse::Ok().json(get_suggestions(db_conn.get_ref()).await?))
}

/// Gets a pending suggestion by id.
/// This Endpoint requires Credentials of at least Automation level.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "Suggestion",
	security(
        ("bearer_auth" = [])
	),
	responses(
		(status = 200, description = "Successfully retrieved suggestions", body = Suggestion),
		(status = 401, description = "Unauthorized, you need to be logged in to view suggestion"),
		(status = 403, description = "Forbidden, you do not have permission to view suggestions"),
	)
)]
#[get("/suggestion/{id}")]
pub async fn get_suggestion_by_id(
	id: Path<Uuid>,
	db_conn: Data<DatabaseConnection>,
	req: HttpRequest,
) -> error::Result<impl Responder> {
	handle_auth_and_permissions(&UserPermissionsEnum::Automation, req, db_conn.clone()).await?;

	Ok(HttpResponse::Ok().json(get_suggestion(id.into_inner(), db_conn.get_ref()).await?))
}

/// Adds a suggestion for a manual game metadata match.
/// This Endpoint requires Credentials of a User.
#[utoipa::path(
	post,
	context_path = "/api",
	tag = "Suggestion",
	security(
        ("bearer_auth" = [])
	),
	responses(
		(status = 200, description = "Successfully created suggestion", body = Suggestion),
		(status = 401, description = "Unauthorized, you need to be logged in to create a suggestion"),
		(status = 404, description = "Could not find a Game with the provided id"),
		(status = 409, description = "A suggestion for this game already exists with the same provider & provider id.")
	)
)]
#[post("/suggestion/game")]
pub async fn create_game_suggestion(
	body: Json<GameSuggestionRequest>,
	db_conn: Data<DatabaseConnection>,
	req: HttpRequest,
) -> error::Result<impl Responder> {
	handle_auth_and_permissions(&UserPermissionsEnum::User, req, db_conn.clone()).await?;

	Ok(HttpResponse::Ok().json(add_game_suggestion(body.into_inner(), db_conn.get_ref()).await?))
}

/// Adds a suggestion for a manual platform metadata match.
/// This Endpoint requires Credentials of a User.
#[utoipa::path(
	post,
	context_path = "/api",
	tag = "Suggestion",
	security(
        ("bearer_auth" = [])
	),
	responses(
		(status = 200, description = "Successfully created suggestion", body = Suggestion),
		(status = 401, description = "Unauthorized, you need to be logged in to create a suggestion"),
		(status = 404, description = "Could not find a Platform with the provided id"),
		(status = 409, description = "A suggestion for this Platform already exists with the same provider & provider id.")
	)
)]
#[post("/suggestion/platform")]
pub async fn create_platform_suggestion(
	body: Json<CompanyOrPlatformSuggestionRequest>,
	db_conn: Data<DatabaseConnection>,
	req: HttpRequest,
) -> error::Result<impl Responder> {
	handle_auth_and_permissions(&UserPermissionsEnum::User, req, db_conn.clone()).await?;

	Ok(HttpResponse::Ok()
		.json(add_platform_suggestion(body.into_inner(), db_conn.get_ref()).await?))
}

/// Adds a suggestion for a manual company metadata match.
/// This Endpoint requires Credentials of a User.
#[utoipa::path(
	post,
	context_path = "/api",
	tag = "Suggestion",
	security(
        ("bearer_auth" = [])
	),
	responses(
		(status = 200, description = "Successfully created suggestion", body = Suggestion),
		(status = 401, description = "Unauthorized, you need to be logged in to create a suggestion"),
		(status = 404, description = "Could not find a Company with the provided id"),
		(status = 409, description = "A suggestion for this Company already exists with the same provider & provider id.")
	)
)]
#[post("/suggestion/company")]
pub async fn create_company_suggestion(
	body: Json<CompanyOrPlatformSuggestionRequest>,
	db_conn: Data<DatabaseConnection>,
	req: HttpRequest,
) -> error::Result<impl Responder> {
	handle_auth_and_permissions(&UserPermissionsEnum::User, req, db_conn.clone()).await?;

	Ok(
		HttpResponse::Ok()
			.json(add_company_suggestion(body.into_inner(), db_conn.get_ref()).await?),
	)
}

/// Approves a suggestion by id.
/// This Endpoint requires Credentials of at least Automation level.
#[utoipa::path(
	post,
	context_path = "/api",
	tag = "Suggestion",
	security(
        ("bearer_auth" = [])
	),
	responses(
		(status = 200, description = "Successfully approved suggestion", body = UpdatedMetadataMatchesFromSuggestionResponse),
		(status = 401, description = "Unauthorized, you need to be logged in to approve a suggestion"),
		(status = 403, description = "Forbidden, you do not have permission to approve suggestions"),
		(status = 404, description = "Could not find a Suggestion with the provided id"),
	)
)]
#[post("/suggestion/{id}/accept")]
pub async fn approve_suggestion(
	id: Path<Uuid>,
	db_conn: Data<DatabaseConnection>,
	redis_client: Data<redis::Client>,
	req: HttpRequest,
) -> error::Result<impl Responder> {
	handle_auth_and_permissions(&UserPermissionsEnum::Automation, req, db_conn.clone()).await?;

	let updated = accept_suggestion(
		id.into_inner(),
		db_conn.get_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
	)
	.await?;

	Ok(HttpResponse::Ok().json(UpdatedMetadataMatchesFromSuggestionResponse { updated }))
}

/// Declines a suggestion by id.
/// This Endpoint requires Credentials of at least Automation level.
#[utoipa::path(
	delete,
	context_path = "/api",
	tag = "Suggestion",
	security(
        ("bearer_auth" = [])
	),
	responses(
		(status = 204, description = "Successfully declined suggestion"),
		(status = 401, description = "Unauthorized, you need to be logged in to decline a suggestion"),
		(status = 403, description = "Forbidden, you do not have permission to decline suggestions"),
		(status = 404, description = "Could not find a Suggestion with the provided id"),
	)
)]
#[delete("/suggestion/{id}")]
pub async fn delete_suggestion(
	id: Path<Uuid>,
	db_conn: Data<DatabaseConnection>,
	req: HttpRequest,
) -> error::Result<impl Responder> {
	handle_auth_and_permissions(&UserPermissionsEnum::Automation, req, db_conn.clone()).await?;

	decline_suggestion(id.into_inner(), db_conn.get_ref()).await?;

	Ok(HttpResponse::NoContent())
}
