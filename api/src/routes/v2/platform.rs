use crate::error;
use crate::model::pagination::{FNV_OFFSET, PageParams, fnv1a};
use crate::routes::v2::error::ok_or_v2_not_found;
use crate::routes::v2::{
	build_page, require_non_empty_query, resolve_keyset_start, resolve_or_return,
};
use actix_web::web::{Data, Path, Query};
use actix_web::{Responder, get};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use service::db::platform::search_platforms_by_name_page;
use service::entities::platform::{
	count_all_platforms, find_platforms_page_and_related_company_and_signature_metadata_mapping,
	get_platform_by_id_and_related_company_and_signature_metadata_mapping,
};
use service::model::PlatformMetadataResponse;
use utoipa::IntoParams;
use uuid::Uuid;

const PLATFORM_FILTER_TAG: u64 = 0x706c_6174_666f_726d;

/// Base discriminator for platform name-search cursors. The search query is
/// folded into this so a cursor minted under one query is rejected under another.
const PLATFORM_SEARCH_FILTER_TAG_BASE: u64 = 0x706c_6174_7372_6300;

fn platform_search_filter_tag(query: &str) -> u64 {
	PLATFORM_SEARCH_FILTER_TAG_BASE ^ fnv1a(FNV_OFFSET, query.as_bytes())
}

/// The search literal for the v2 keyset-paginated platform name search.
/// Pagination is supplied separately via the shared `PageParams`.
#[derive(Debug, Serialize, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct PlatformSearchQuery {
	/// The platform name to search for.
	pub query: String,
}

/// Lists platforms ordered by name.
///
/// Each entry carries the platform with its company and external metadata mappings.
#[utoipa::path(
	get,
	tag = "Platform",
	params(PageParams),
	responses(
		(status = 200, description = "One page of platforms ordered by name", body = PageOfPlatformMetadataResponse),
		(status = 400, description = "Malformed pagination cursor or filter-set mismatch", body = V2ErrorBody)
	)
)]
#[get("/platforms")]
pub async fn list_platforms_v2(
	params: Query<PageParams>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let params = params.into_inner();
	let limit = params.limit_clamped();
	let start = resolve_or_return!(resolve_keyset_start(&params, PLATFORM_FILTER_TAG));
	let has_previous = start.has_previous();

	let page = find_platforms_page_and_related_company_and_signature_metadata_mapping(
		start.after(),
		Some(limit),
		db_conn.get_ref(),
	)
	.await?;

	let total = if params.wants_total() {
		Some(count_all_platforms(db_conn.get_ref()).await?)
	} else {
		None
	};

	Ok(build_page(
		page.rows,
		page.has_more,
		has_previous,
		limit,
		PLATFORM_FILTER_TAG,
		total,
	))
}

/// Searches platforms by name, ordered by name.
///
/// Case-insensitive substring match on the platform name. Each entry carries the platform with its company and external metadata mappings. Returns an empty array when nothing matches.
#[utoipa::path(
	get,
	tag = "Platform",
	params(PlatformSearchQuery, PageParams),
	responses(
		(status = 200, description = "One page of matching platforms ordered by name", body = PageOfPlatformMetadataResponse),
		(status = 400, description = "Empty search query, or a malformed pagination cursor or filter-set mismatch", body = V2ErrorBody)
	)
)]
#[get("/platforms/search")]
pub async fn search_platforms_v2(
	query: Query<PlatformSearchQuery>,
	params: Query<PageParams>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let query = query.into_inner();
	let literal = resolve_or_return!(require_non_empty_query(&query.query));
	let params = params.into_inner();
	let limit = params.limit_clamped();
	let filter_tag = platform_search_filter_tag(literal);
	let start = resolve_or_return!(resolve_keyset_start(&params, filter_tag));
	let has_previous = start.has_previous();

	let page =
		search_platforms_by_name_page(literal, start.after(), Some(limit), db_conn.get_ref())
			.await?;

	let rows: Vec<PlatformMetadataResponse> = page
		.rows
		.into_iter()
		.map(|(platform, company, mappings)| PlatformMetadataResponse {
			id: platform.id,
			name: platform.name,
			company_id: company.clone().map(|company| company.id),
			company_name: company.map(|company| company.name),
			external_metadata: mappings.into_iter().map(Into::into).collect(),
		})
		.collect();

	Ok(build_page(
		rows,
		page.has_more,
		has_previous,
		limit,
		filter_tag,
		None,
	))
}

/// Returns a platform by id.
///
/// The response includes the platform with its company and metadata mappings.
#[utoipa::path(
	get,
	tag = "Platform",
	responses(
		(status = 200, description = "The platform and its metadata mappings", body = PlatformMetadataResponse),
		(status = 404, description = "Platform not found", body = V2ErrorBody)
	)
)]
#[get("/platforms/{id}")]
pub async fn get_platform_by_id_v2(
	id: Path<Uuid>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let platform_response = get_platform_by_id_and_related_company_and_signature_metadata_mapping(
		id.into_inner(),
		db_conn.get_ref(),
	)
	.await?;

	Ok(ok_or_v2_not_found(
		platform_response,
		"platform_not_found",
		"platform not found",
	))
}
