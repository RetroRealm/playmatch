use crate::error;
use actix_web::web::{Data, Path};
use actix_web::{get, HttpResponse, Responder};
use sea_orm::DatabaseConnection;
use service::platform::{
	find_all_and_related_company_and_signature_metadata_mapping,
	get_platform_by_id_and_related_company_and_signature_metadata_mapping,
};
use uuid::Uuid;

/// Returns all platforms with its company and its external metadata mappings.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "Platform",
	responses(
		(status = 200, description = "Returns a list of Platforms playmatch knows about including its company and metadata mappings", body = Vec<PlatformResponse>)
	)
)]
#[get("/platforms")]
pub async fn get_all_platforms(db_conn: Data<DatabaseConnection>) -> error::Result<impl Responder> {
	let companies =
		find_all_and_related_company_and_signature_metadata_mapping(db_conn.get_ref()).await?;

	Ok(HttpResponse::Ok().json(companies))
}

/// Returns a platform and its metadata mappings by id.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "Platform",
	responses(
		(status = 200, description = "Returns a Platform and its metadata mappings", body = PlatformResponse),
		(status = 404, description = "Platform not found")
	)
)]
#[get("/platforms/{id}")]
pub async fn get_platform_by_id(
	id: Path<Uuid>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let platform_response = get_platform_by_id_and_related_company_and_signature_metadata_mapping(
		id.into_inner(),
		db_conn.get_ref(),
	)
	.await?;

	if let Some(platform) = platform_response {
		Ok(HttpResponse::Ok().json(platform))
	} else {
		Ok(HttpResponse::NotFound().finish())
	}
}
