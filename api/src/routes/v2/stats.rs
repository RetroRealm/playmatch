use crate::error;
use crate::routes::v2::error::ok_or_v2_not_found;
use actix_web::web::{Data, Path};
use actix_web::{HttpResponse, Responder, get};
use sea_orm::DatabaseConnection;
use service::entities::stats::{get_platform_stats, get_service_stats};
use uuid::Uuid;

/// Returns aggregate counts across the whole service.
///
/// The counts change only when a dat file is imported.
#[utoipa::path(
	get,
	tag = "Stats",
	responses(
		(status = 200, description = "Aggregate counts across the whole service", body = ServiceStats)
	)
)]
#[get("/stats")]
pub async fn get_stats_v2(db_conn: Data<DatabaseConnection>) -> error::Result<impl Responder> {
	let stats = get_service_stats(db_conn.get_ref()).await?;
	Ok(HttpResponse::Ok().json(stats))
}

/// Returns aggregate counts for a single platform.
#[utoipa::path(
	get,
	tag = "Stats",
	responses(
		(status = 200, description = "Aggregate counts for a single platform", body = PlatformStats),
		(status = 404, description = "Platform not found", body = V2ErrorBody)
	)
)]
#[get("/platforms/{id}/stats")]
pub async fn get_platform_stats_v2(
	id: Path<Uuid>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let stats = get_platform_stats(id.into_inner(), db_conn.get_ref()).await?;
	Ok(ok_or_v2_not_found(
		stats,
		"platform_not_found",
		"platform not found",
	))
}
