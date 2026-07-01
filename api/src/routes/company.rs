use crate::error;
use crate::routes::ok_or_not_found;
use actix_web::web::{Data, Path};
use actix_web::{HttpResponse, Responder, get};
use sea_orm::DatabaseConnection;
use service::entities::company::{
	find_all_companies_and_external_metadata, get_company_by_id_and_external_metadata,
};
use uuid::Uuid;

/// Lists all companies with their external metadata mappings.
///
/// Each company carries the metadata provider mappings known for it.
#[utoipa::path(
	get,
	tag = "Company",
	responses(
		(status = 200, description = "The full list of companies, each with its external metadata mappings", body = Vec<CompanyMetadataResponse>)
	)
)]
#[get("/companies")]
pub async fn get_all_companies(db_conn: Data<DatabaseConnection>) -> error::Result<impl Responder> {
	let companies_response = find_all_companies_and_external_metadata(db_conn.get_ref()).await?;

	Ok(HttpResponse::Ok().json(companies_response))
}

/// Returns a company by id.
///
/// The company carries the metadata provider mappings known for it.
#[utoipa::path(
	get,
	tag = "Company",
	responses(
		(status = 200, description = "The company and its external metadata mappings", body = CompanyMetadataResponse),
		(status = 404, description = "Company not found")
	)
)]
#[get("/companies/{id}")]
pub async fn get_company_by_id(
	id: Path<Uuid>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let company_response =
		get_company_by_id_and_external_metadata(id.into_inner(), db_conn.get_ref()).await?;

	Ok(ok_or_not_found(company_response))
}
