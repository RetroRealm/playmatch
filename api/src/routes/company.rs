use crate::error;
use actix_web::web::Data;
use actix_web::{get, HttpResponse, Responder};
use sea_orm::DatabaseConnection;
use service::company::find_all_companies_and_external_metadata;

/// Returns all companies and its external metadata mappings.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "Company",
	responses(
		(status = 200, description = "Returns a list of Companies playmatch knows about including its metadata mappings", body = Vec<CompanyResponse>)
	)
)]
#[get("/company")]
pub async fn get_all_companies(db_conn: Data<DatabaseConnection>) -> error::Result<impl Responder> {
	let companies = find_all_companies_and_external_metadata(db_conn.get_ref()).await?;

	Ok(HttpResponse::Ok().json(companies))
}
