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

/// Lists all pending suggestions.
///
/// Requires credentials of at least Automation level.
#[utoipa::path(
	get,
	tag = "Suggestion",
	security(
        ("bearer_auth" = [])
	),
	responses(
		(status = 200, description = "All pending suggestions", body = Vec<Suggestion>),
		(status = 401, description = "Missing or invalid credentials"),
		(status = 403, description = "Credentials below Automation level"),
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

/// Returns a pending suggestion by id.
///
/// Requires credentials of at least Automation level.
#[utoipa::path(
	get,
	tag = "Suggestion",
	security(
        ("bearer_auth" = [])
	),
	responses(
		(status = 200, description = "The requested suggestion", body = Suggestion),
		(status = 401, description = "Missing or invalid credentials"),
		(status = 403, description = "Credentials below Automation level"),
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

/// Creates a suggestion for a game metadata match.
///
/// Proposes external metadata for a game so a reviewer can approve or decline it. Requires credentials of at least User level.
#[utoipa::path(
	post,
	tag = "Suggestion",
	security(
        ("bearer_auth" = [])
	),
	responses(
		(status = 200, description = "The created suggestion", body = Suggestion),
		(status = 401, description = "Missing or invalid credentials"),
		(status = 404, description = "Game not found"),
		(status = 409, description = "A suggestion for this game with the same provider and provider id already exists")
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

/// Creates a suggestion for a platform metadata match.
///
/// Proposes external metadata for a platform so a reviewer can approve or decline it. Requires credentials of at least User level.
#[utoipa::path(
	post,
	tag = "Suggestion",
	security(
        ("bearer_auth" = [])
	),
	responses(
		(status = 200, description = "The created suggestion", body = Suggestion),
		(status = 401, description = "Missing or invalid credentials"),
		(status = 404, description = "Platform not found"),
		(status = 409, description = "A suggestion for this platform with the same provider and provider id already exists")
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

/// Creates a suggestion for a company metadata match.
///
/// Proposes external metadata for a company so a reviewer can approve or decline it. Requires credentials of at least User level.
#[utoipa::path(
	post,
	tag = "Suggestion",
	security(
        ("bearer_auth" = [])
	),
	responses(
		(status = 200, description = "The created suggestion", body = Suggestion),
		(status = 401, description = "Missing or invalid credentials"),
		(status = 404, description = "Company not found"),
		(status = 409, description = "A suggestion for this company with the same provider and provider id already exists")
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
///
/// Applies the suggested external metadata to its target. Requires credentials of at least Automation level.
#[utoipa::path(
	post,
	tag = "Suggestion",
	security(
        ("bearer_auth" = [])
	),
	responses(
		(status = 200, description = "The metadata matches updated from the approved suggestion", body = UpdatedMetadataMatchesFromSuggestionResponse),
		(status = 401, description = "Missing or invalid credentials"),
		(status = 403, description = "Credentials below Automation level"),
		(status = 404, description = "Suggestion not found"),
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

/// Submits a game match suggestion from a third-party tool.
///
/// Queues the proposed mapping for later processing and returns 204 without waiting on the result. The payload is validated in the background, and a suggestion is created only for a ROM already in the database whose proposed mapping is not yet recorded. No credentials are required. Submissions are rate limited per client IP.
#[utoipa::path(
	post,
	tag = "Suggestion",
	responses(
		(status = 204, description = "The submission was accepted for background processing"),
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
				log::warn!("External suggestion rate limit check failed for {ip}: {e}");
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
			log::warn!("External suggestion enqueue failed: {e}");
			service::metrics::record_user_action("external_suggestion", "bad_payload");
		}
	}

	Ok(HttpResponse::NoContent())
}

/// Declines a suggestion by id.
///
/// Removes the pending suggestion without applying it. Requires credentials of at least Automation level.
#[utoipa::path(
	delete,
	tag = "Suggestion",
	security(
        ("bearer_auth" = [])
	),
	responses(
		(status = 204, description = "The suggestion was declined"),
		(status = 401, description = "Missing or invalid credentials"),
		(status = 403, description = "Credentials below Automation level"),
		(status = 404, description = "Suggestion not found"),
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
