use crate::error;
use actix_web::web::{Data, Path};
use actix_web::{get, HttpResponse, Responder};
use sea_orm::DatabaseConnection;
use service::company::{
	find_all_companies_and_external_metadata, get_company_by_id_and_external_metadata,
};
use uuid::Uuid;

/// Returns all companies and its external metadata mappings.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "Company",
	responses(
		(status = 200, description = "Returns a list of Companies playmatch knows about including its metadata mappings", body = Vec<CompanyResponse>)
	)
)]
#[get("/companies")]
pub async fn get_all_companies(db_conn: Data<DatabaseConnection>) -> error::Result<impl Responder> {
	let companies_response = find_all_companies_and_external_metadata(db_conn.get_ref()).await?;

	Ok(HttpResponse::Ok().json(companies_response))
}

/// Returns a company and its metadata mappings by id.
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "Company",
	responses(
		(status = 200, description = "Returns a Company and its metadata mappings", body = CompanyResponse),
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

	if let Some(company) = company_response {
		Ok(HttpResponse::Ok().json(company))
	} else {
		Ok(HttpResponse::NotFound().finish())
	}
}
