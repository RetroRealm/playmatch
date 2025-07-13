use crate::error;
use crate::routes::handle_auth_and_permissions;
use actix_web::web::{Data, Json};
use actix_web::{HttpRequest, HttpResponse, Responder, post};
use entity::sea_orm_active_enums::UserPermissionsEnum;
use sea_orm::DatabaseConnection;
use service::model::suggestion::GameSuggestionRequest;
use service::suggestion::add_suggestion;

/// This Endpoint requires Credentials of a User.
/// Find a User by their ID.
#[utoipa::path(
	post,
	context_path = "/api",
	tag = "Suggestion",
	security(
        ("bearer_auth" = [])
	),
	responses(
		(status = 200, description = "Successfully created suggestion", body = User),
		(status = 404, description = "Could not find Game with the provided id"),
		(status = 409, description = "A suggestion for this game already exists with the same provider id.")
	)
)]
#[post("/suggestion/game")]
pub async fn add_game_suggestion(
	_body: Json<GameSuggestionRequest>,
	db_conn: Data<DatabaseConnection>,
	req: HttpRequest,
) -> error::Result<impl Responder> {
	handle_auth_and_permissions(&UserPermissionsEnum::User, req, db_conn.clone()).await?;

	Ok(HttpResponse::Ok().json(add_suggestion().await?))
}
