use crate::error;
use crate::routes::handle_auth_and_permissions;
use crate::util::http::client_ip_from_http_request;
use actix_web::web::{Data, Json, Path};
use actix_web::{HttpRequest, HttpResponse, Responder, delete, get, post};
use entity::sea_orm_active_enums::UserPermissionsEnum;
use redis::aio::MultiplexedConnection;
use sea_orm::DatabaseConnection;
use service::external_suggestion::{
	EnqueueOutcome, check_rate_limit, enqueue_external_suggestion, truncate_user_agent,
};
use service::matching::suggestions::{
	accept_suggestion, add_company_suggestion, add_game_suggestion, add_platform_suggestion,
	decline_suggestion, get_suggestion, get_suggestions,
};
use service::model::external_suggestion::ExternalGameMatchSuggestionPayload;
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
	redis_conn: Data<MultiplexedConnection>,
	req: HttpRequest,
) -> error::Result<impl Responder> {
	handle_auth_and_permissions(&UserPermissionsEnum::Automation, req, db_conn.clone()).await?;

	let mut redis_conn = redis_conn.get_ref().clone();
	let updated = accept_suggestion(id.into_inner(), db_conn.get_ref(), &mut redis_conn).await?;

	Ok(HttpResponse::Ok().json(UpdatedMetadataMatchesFromSuggestionResponse { updated }))
}

/// Fire-and-forget signal from third-party tools. Always returns 204; a scheduled
/// worker validates the payload and only creates a suggestion for ROMs already in
/// the database where the proposed mapping is not yet represented.
#[utoipa::path(
	post,
	context_path = "/api",
	tag = "Suggestion",
	responses(
		(status = 204, description = "Accepted. Processing happens asynchronously."),
	)
)]
#[post("/suggestion/external/game")]
pub async fn submit_external_game_suggestion(
	body: Json<ExternalGameMatchSuggestionPayload>,
	redis_conn: Data<MultiplexedConnection>,
	req: HttpRequest,
) -> error::Result<impl Responder> {
	let mut redis_conn = redis_conn.get_ref().clone();

	if let Some(ip) = client_ip_from_http_request(&req) {
		let ip = ip.to_string();
		match check_rate_limit(&mut redis_conn, &ip).await {
			Ok(true) => {}
			Ok(false) => {
				service::metrics::record_user_action("external_suggestion", "rate_limited");
				return Ok(HttpResponse::NoContent());
			}
			Err(e) => {
				log::warn!("external suggestion rate limit check failed for {ip}: {e}");
			}
		}
	}

	let user_agent = truncate_user_agent(
		req.headers()
			.get("User-Agent")
			.and_then(|h| h.to_str().ok()),
	);

	match enqueue_external_suggestion(&mut redis_conn, body.into_inner(), user_agent).await {
		Ok(EnqueueOutcome::Accepted) => {
			service::metrics::record_user_action("external_suggestion", "accepted");
		}
		Ok(EnqueueOutcome::RateLimited) => {
			service::metrics::record_user_action("external_suggestion", "rate_limited");
		}
		Ok(EnqueueOutcome::QueueFull) => {
			service::metrics::record_user_action("external_suggestion", "queue_full");
		}
		Err(e) => {
			log::warn!("external suggestion enqueue failed: {e}");
			service::metrics::record_user_action("external_suggestion", "bad_payload");
		}
	}

	Ok(HttpResponse::NoContent())
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
