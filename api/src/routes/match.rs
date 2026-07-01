use crate::error;
use crate::routes::handle_auth_and_permissions;
use actix_web::web::{Data, Json};
use actix_web::{HttpRequest, HttpResponse, Responder, post};
use entity::sea_orm_active_enums::UserPermissionsEnum;
use log::debug;
use redis::aio::MultiplexedConnection;
use sea_orm::DatabaseConnection;
use service::matching::manual::{
	apply_manual_company_match, apply_manual_game_match, apply_manual_platform_match,
};
use service::model::ManualMatchMode;
use service::model::matching::{CompanyOrPlatformMatchRequest, GameMatchRequest, MatchRequest};

/// Matches a game to external metadata by file hash, filename, or name.
///
/// Identify the game by any of its file hashes, its filename, or its game name, then attach the chosen external metadata to it. The strongest supplied hash resolves the file: sha256, then sha1, then md5, then crc. At least one of `file_name`, `md5`, `sha1`, or `sha256` is required. Requires credentials of at least Trusted level. Callers without a Trusted account use the suggestion endpoints instead.
#[utoipa::path(
	post,
	tag = "Match",
	security(
        ("bearer_auth" = [])
	),
	responses(
		(status = 200, description = "The matched game and its updated external metadata", body = Vec<UpdatedMatchResult>),
		(status = 400, description = "None of file_name, md5, sha1, or sha256 supplied, or a supplied hash is malformed"),
		(status = 401, description = "Missing or invalid credentials"),
		(status = 403, description = "Credentials below Trusted level"),
		(status = 404, description = "Game not found")
	)
)]
#[post("/match/manual/game")]
pub async fn manually_match_game(
	match_request: Json<GameMatchRequest>,
	db_conn: Data<DatabaseConnection>,
	redis_conn: Data<MultiplexedConnection>,
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

	if let Err(msg) = match_request.validate() {
		return Err(error::Error::BadRequest(msg));
	}

	if match_request.name.is_none()
		&& match_request.md5.is_none()
		&& match_request.sha1.is_none()
		&& match_request.sha256.is_none()
	{
		return Err(error::Error::BadRequest(
			"At least one of file_name, md5, sha1 or sha256 must be provided.".to_string(),
		));
	}

	let mut redis_conn = redis_conn.get_ref().clone();
	let updated =
		apply_manual_game_match(match_request, db_conn.get_ref(), &mut redis_conn).await?;

	Ok(HttpResponse::Ok().json(updated))
}

/// Matches a platform to external metadata by name.
///
/// Identify the platform by its name and attach the chosen external metadata to it. Requires credentials of at least Trusted level. Callers without a Trusted account use the suggestion endpoints instead.
#[utoipa::path(
	post,
	tag = "Match",
	security(
        ("bearer_auth" = [])
	),
	responses(
		(status = 200, description = "The matched platform and its updated external metadata", body = UpdatedMatchResult),
		(status = 401, description = "Missing or invalid credentials"),
		(status = 403, description = "Credentials below Trusted level"),
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

	if let Err(msg) = match_request.validate() {
		return Err(error::Error::BadRequest(msg));
	}

	let updated = apply_manual_platform_match(match_request, db_conn.get_ref()).await?;

	Ok(HttpResponse::Ok().json(updated))
}

/// Matches a company to external metadata by name.
///
/// Identify the company by its name and attach the chosen external metadata to it. Requires credentials of at least Trusted level. Callers without a Trusted account use the suggestion endpoints instead.
#[utoipa::path(
	post,
	tag = "Match",
	security(
        ("bearer_auth" = [])
	),
	responses(
		(status = 200, description = "The matched company and its updated external metadata", body = UpdatedMatchResult),
		(status = 401, description = "Missing or invalid credentials"),
		(status = 403, description = "Credentials below Trusted level"),
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

	if let Err(msg) = match_request.validate() {
		return Err(error::Error::BadRequest(msg));
	}

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
