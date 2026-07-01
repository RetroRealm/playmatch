use crate::error;
use crate::routes::ok_or_not_found;
use actix_web::web::{Data, Path};
use actix_web::{HttpResponse, Responder, get};
use sea_orm::DatabaseConnection;
use service::entities::platform::{
	find_all_and_related_company_and_signature_metadata_mapping,
	get_platform_by_id_and_related_company_and_signature_metadata_mapping,
};
use uuid::Uuid;

/// Lists all platforms with their company and external metadata mappings.
///
/// Each platform carries its owning company and the metadata provider mappings known for it.
#[utoipa::path(
	get,
	tag = "Platform",
	responses(
		(status = 200, description = "The full list of platforms, each with its company and external metadata mappings", body = Vec<PlatformMetadataResponse>)
	)
)]
#[get("/platforms")]
pub async fn get_all_platforms(db_conn: Data<DatabaseConnection>) -> error::Result<impl Responder> {
	let companies =
		find_all_and_related_company_and_signature_metadata_mapping(db_conn.get_ref()).await?;

	Ok(HttpResponse::Ok().json(companies))
}

/// Returns a platform by id.
///
/// The platform carries its owning company and the metadata provider mappings known for it.
#[utoipa::path(
	get,
	tag = "Platform",
	responses(
		(status = 200, description = "The platform with its company and external metadata mappings", body = PlatformMetadataResponse),
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

	Ok(ok_or_not_found(platform_response))
}
