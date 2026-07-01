use crate::error;
use crate::model::pagination::PageParams;
use crate::routes::v2::error::ok_or_v2_not_found;
use crate::routes::v2::{
	build_page, require_non_empty_query, resolve_keyset_start, resolve_or_return,
};
use actix_web::web::{Data, Path, Query};
use actix_web::{Responder, get};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use service::db::company::search_companies_by_name_page;
use service::entities::company::{
	find_companies_page_and_external_metadata, get_company_by_id_and_external_metadata,
};
use service::model::CompanyMetadataResponse;
use utoipa::IntoParams;
use uuid::Uuid;

/// Distinguishes this endpoint's cursors from every other v2 list so a cursor
/// minted here cannot be replayed against a different sort+filter set.
const COMPANY_FILTER_TAG: u64 = 0x636f_6d70_616e_7901;

/// Separate cursor space for the name-search list so a browse cursor can never be
/// replayed against the filtered search and vice versa.
const COMPANY_SEARCH_FILTER_TAG: u64 = 0x636f_6d70_7372_6301;

/// The substring to match company names against. Pagination is supplied
/// separately via the shared [`PageParams`].
#[derive(Debug, Serialize, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct CompanySearchQuery {
	/// The case-insensitive substring to match against company names.
	pub query: String,
}

/// Lists companies ordered by name.
///
/// Each entry carries the company and its external metadata mappings.
#[utoipa::path(
	get,
	tag = "Company",
	params(PageParams),
	responses(
		(status = 200, description = "One page of companies ordered by name", body = PageOfCompanyMetadataResponse),
		(status = 400, description = "Malformed pagination cursor or filter-set mismatch", body = V2ErrorBody)
	)
)]
#[get("/companies")]
pub async fn list_companies_v2(
	params: Query<PageParams>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let params = params.into_inner();
	let limit = params.limit_clamped();
	let start = resolve_or_return!(resolve_keyset_start(&params, COMPANY_FILTER_TAG));
	let has_previous = start.has_previous();

	let page =
		find_companies_page_and_external_metadata(start.after(), Some(limit), db_conn.get_ref())
			.await?;

	// companies is write-hot, so with_total is ignored here by design.
	Ok(build_page(
		page.rows,
		page.has_more,
		has_previous,
		limit,
		COMPANY_FILTER_TAG,
		None,
	))
}

/// Searches companies by name, ordered by name.
///
/// Case-insensitive substring match on the company name. Each entry carries the company and its external metadata mappings. Returns an empty array when nothing matches.
#[utoipa::path(
	get,
	tag = "Company",
	params(CompanySearchQuery, PageParams),
	responses(
		(status = 200, description = "One page of matching companies ordered by name", body = PageOfCompanyMetadataResponse),
		(status = 400, description = "Empty search query, or a malformed pagination cursor or filter-set mismatch", body = V2ErrorBody)
	)
)]
#[get("/companies/search")]
pub async fn search_companies_v2(
	query: Query<CompanySearchQuery>,
	params: Query<PageParams>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let query = query.into_inner();
	let literal = resolve_or_return!(require_non_empty_query(&query.query));

	let params = params.into_inner();
	let limit = params.limit_clamped();
	let start = resolve_or_return!(resolve_keyset_start(&params, COMPANY_SEARCH_FILTER_TAG));
	let has_previous = start.has_previous();

	let page =
		search_companies_by_name_page(literal, start.after(), Some(limit), db_conn.get_ref())
			.await?;

	let rows: Vec<CompanyMetadataResponse> = page
		.rows
		.into_iter()
		.map(|(company, mappings)| CompanyMetadataResponse {
			id: company.id,
			name: company.name,
			external_metadata: mappings.into_iter().map(Into::into).collect(),
		})
		.collect();

	Ok(build_page(
		rows,
		page.has_more,
		has_previous,
		limit,
		COMPANY_SEARCH_FILTER_TAG,
		None,
	))
}

/// Returns a company by id.
///
/// The response includes the company and its external metadata mappings.
#[utoipa::path(
	get,
	tag = "Company",
	responses(
		(status = 200, description = "The company and its metadata mappings", body = CompanyMetadataResponse),
		(status = 404, description = "Company not found", body = V2ErrorBody)
	)
)]
#[get("/companies/{id}")]
pub async fn get_company_by_id_v2(
	id: Path<Uuid>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let company_response =
		get_company_by_id_and_external_metadata(id.into_inner(), db_conn.get_ref()).await?;

	Ok(ok_or_v2_not_found(
		company_response,
		"company_not_found",
		"company not found",
	))
}
