use crate::error;
use crate::routes::handle_auth_and_permissions;
use actix_web::web::{Data, Json};
use actix_web::{HttpRequest, HttpResponse, Responder, post};
use entity::sea_orm_active_enums::UserPermissionsEnum;
use log::debug;
use sea_orm::DatabaseConnection;
use service::matching::manual::{
	apply_manual_company_match, apply_manual_game_match, apply_manual_platform_match,
};
use service::model::ManualMatchMode;
use service::model::matching::{CompanyOrPlatformMatchRequest, GameMatchRequest, MatchRequest};

/// Manually match a Game by its file hashes, filename or Game name, returning the matched game ExternalMetadata.
/// This Endpoint requires Credentials of a User with at least Trusted level, if you do not have user credentials and a user account with at least Trusted level, use the suggestion endpoints instead.
#[utoipa::path(
	post,
	context_path = "/api",
	tag = "Match",
	security(
        ("bearer_auth" = [])
	),
	responses(
		(status = 200, description = "Successfully Matched Game", body = Vec<UpdatedMatchResult>),
		(status = 400, description = "At least one of file_name, md5, sha1 or sha256 must be provided."),
		(status = 401, description = "Unauthorized, you need to be logged in to manually match a game"),
		(status = 403, description = "Forbidden, you do not have permission to manually match a game"),
		(status = 404, description = "Game not found")
	)
)]
#[post("/match/manual/game")]
pub async fn manually_match_game(
	match_request: Json<GameMatchRequest>,
	db_conn: Data<DatabaseConnection>,
	redis_client: Data<redis::Client>,
	req: HttpRequest,
) -> error::Result<impl Responder> {
	let mut match_request = match_request.into_inner();
	handle_auth_and_permissions_match(
		UserPermissionsEnum::Trusted,
		&mut match_request,
		req,
		db_conn.clone(),
	)
	.await?;

	if match_request.name.is_none()
		&& match_request.md5.is_none()
		&& match_request.sha1.is_none()
		&& match_request.sha256.is_none()
	{
		return Ok(HttpResponse::BadRequest()
			.body("At least one of file_name, md5, sha1 or sha256 must be provided."));
	}

	let updated = apply_manual_game_match(
		match_request,
		db_conn.get_ref(),
		&mut redis_client.get_multiplexed_async_connection().await?,
	)
	.await?;

	Ok(HttpResponse::Ok().json(updated))
}

/// Manually match a Platform by its name, returning the matched ExternalMetadata.
/// This Endpoint requires Credentials of a User with at least Trusted level, if you do not have user credentials and a user account with at least Trusted level, use the suggestion endpoints instead.
#[utoipa::path(
	post,
	context_path = "/api",
	tag = "Match",
	security(
        ("bearer_auth" = [])
	),
	responses(
		(status = 200, description = "Successfully Matched Platform", body = UpdatedMatchResult),
		(status = 401, description = "Unauthorized, you need to be logged in to manually match a platform"),
		(status = 403, description = "Forbidden, you do not have permission to manually match a platform"),
		(status = 404, description = "Platform not found")
	)
)]
#[post("/match/manual/platform")]
pub async fn manually_match_platform(
	match_request: Json<CompanyOrPlatformMatchRequest>,
	db_conn: Data<DatabaseConnection>,
	req: HttpRequest,
) -> error::Result<impl Responder> {
	let mut match_request = match_request.into_inner();
	handle_auth_and_permissions_match(
		UserPermissionsEnum::Trusted,
		&mut match_request,
		req,
		db_conn.clone(),
	)
	.await?;

	let updated = apply_manual_platform_match(match_request, db_conn.get_ref()).await?;

	Ok(HttpResponse::Ok().json(updated))
}

/// Manually match a Company by its name, returning the matched ExternalMetadata.
/// This Endpoint requires Credentials of a User with at least Trusted level, if you do not have user credentials and a user account with at least Trusted level, use the suggestion endpoints instead.
#[utoipa::path(
	post,
	context_path = "/api",
	tag = "Match",
	security(
        ("bearer_auth" = [])
	),
	responses(
		(status = 200, description = "Successfully Matched Company", body = UpdatedMatchResult),
		(status = 401, description = "Unauthorized, you need to be logged in to manually match a company"),
		(status = 403, description = "Forbidden, you do not have permission to manually match a company"),
		(status = 404, description = "Company not found")
	)
)]
#[post("/match/manual/company")]
pub async fn manually_match_company(
	match_request: Json<CompanyOrPlatformMatchRequest>,
	db_conn: Data<DatabaseConnection>,
	req: HttpRequest,
) -> error::Result<impl Responder> {
	let mut match_request = match_request.into_inner();
	handle_auth_and_permissions_match(
		UserPermissionsEnum::Trusted,
		&mut match_request,
		req,
		db_conn.clone(),
	)
	.await?;

	let updated = apply_manual_company_match(match_request, db_conn.get_ref()).await?;

	Ok(HttpResponse::Ok().json(updated))
}

async fn handle_auth_and_permissions_match(
	required_user_perms: UserPermissionsEnum,
	match_request: &mut impl MatchRequest,
	req: HttpRequest,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<entity::user::Model> {
	let user = handle_auth_and_permissions(&required_user_perms, req, db_conn).await?;

	if user.permissions == UserPermissionsEnum::Trusted
		&& match_request.get_manual_match_type() != ManualMatchMode::Trusted
	{
		debug!("User is Trusted, setting manual match type to Trusted");
		match_request.set_manual_match_type(ManualMatchMode::Trusted);
	}

	if user.permissions != UserPermissionsEnum::Automation
		&& user.permissions != UserPermissionsEnum::Admin
		&& match_request.get_user_id() != Some(user.id)
	{
		debug!(
			"User is not Automation or Admin, setting user_id to {}",
			user.id
		);
		match_request.set_user_id(Some(user.id));
	}

	Ok(user)
}
