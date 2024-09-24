use crate::error;
use actix_web::web::Data;
use actix_web::{get, HttpResponse, Responder};
use sea_orm::DatabaseConnection;
use service::platform::find_all_and_related_company_and_signature_metadata_mapping;

/// Returns all platforms with its company and its external metadata mappings.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "Platform",
	responses(
		(status = 200, description = "Returns a list of Platforms playmatch knows about including its company and metadata mappings", body = Vec<PlatformResponse>)
	)
)]
#[get("/platform")]
pub async fn get_all_platforms(db_conn: Data<DatabaseConnection>) -> error::Result<impl Responder> {
	let companies =
		find_all_and_related_company_and_signature_metadata_mapping(db_conn.get_ref()).await?;

	Ok(HttpResponse::Ok().json(companies))
}
