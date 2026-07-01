use crate::error;
use crate::model::dat_file::{SignatureGroupDatFilesQuery, SignatureGroupGamesQuery};
use crate::model::pagination::{FNV_OFFSET, PageParams, fnv1a, fnv1a_uuid};
use crate::routes::v2::error::{ok_or_v2_not_found, v2_not_found};
use crate::routes::v2::{
	build_page, require_non_empty_query, resolve_keyset_start, resolve_or_return,
};
use actix_web::web::{Data, Path, Query};
use actix_web::{Responder, get};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use service::db::signature_group::search_signature_groups_by_name_page;
use service::entities::signature_group::{
	count_signature_groups, find_signature_group_by_id, find_signature_group_dat_files_page,
	find_signature_group_games_page, find_signature_groups_page,
};
use service::model::PlaymatchSignatureGroupV2;
use utoipa::IntoParams;
use uuid::Uuid;

const SIGNATURE_GROUP_FILTER_TAG: u64 = 0x7369_6774_5f67_7270;
const SIGNATURE_GROUP_DAT_FILES_FILTER_TAG_BASE: u64 = 0x7367_5f64_6174_0000;
const SIGNATURE_GROUP_GAMES_FILTER_TAG_BASE: u64 = 0x7367_5f67_616d_0000;
const SIGNATURE_GROUP_SEARCH_FILTER_TAG_BASE: u64 = 0x7367_5f73_7263_0000;

fn dat_files_filter_tag(signature_group_id: Uuid, platform_id: Option<Uuid>) -> u64 {
	let hash = fnv1a(FNV_OFFSET, signature_group_id.as_bytes());
	let hash = fnv1a_uuid(hash, platform_id);
	SIGNATURE_GROUP_DAT_FILES_FILTER_TAG_BASE ^ hash
}

fn games_filter_tag(
	signature_group_id: Uuid,
	platform_id: Option<Uuid>,
	current_only: bool,
) -> u64 {
	let hash = fnv1a(FNV_OFFSET, signature_group_id.as_bytes());
	let hash = fnv1a_uuid(hash, platform_id);
	SIGNATURE_GROUP_GAMES_FILTER_TAG_BASE ^ fnv1a(hash, &[current_only as u8])
}

fn search_filter_tag(query: &str) -> u64 {
	SIGNATURE_GROUP_SEARCH_FILTER_TAG_BASE ^ fnv1a(FNV_OFFSET, query.as_bytes())
}

/// The search literal for the v2 keyset-paginated signature group search.
/// Pagination is supplied separately via the shared `PageParams`.
#[derive(Debug, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
struct SignatureGroupSearchQuery {
	/// The signature group name to search for.
	query: String,
}

/// Lists signature groups ordered by name.
#[utoipa::path(
	get,
	tag = "Signature Group",
	params(PageParams),
	responses(
		(status = 200, description = "One page of signature groups ordered by name", body = PageOfPlaymatchSignatureGroup),
		(status = 400, description = "Malformed pagination cursor or filter-set mismatch", body = V2ErrorBody)
	)
)]
#[get("/signature-groups")]
pub async fn list_signature_groups_v2(
	params: Query<PageParams>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let params = params.into_inner();
	let limit = params.limit_clamped();
	let start = resolve_or_return!(resolve_keyset_start(&params, SIGNATURE_GROUP_FILTER_TAG));
	let has_previous = start.has_previous();

	let page = find_signature_groups_page(start.after(), Some(limit), db_conn.get_ref()).await?;
	let page = page.map_rows(PlaymatchSignatureGroupV2::from);

	let total = if params.wants_total() {
		Some(count_signature_groups(db_conn.get_ref()).await?)
	} else {
		None
	};

	Ok(build_page(
		page.rows,
		page.has_more,
		has_previous,
		limit,
		SIGNATURE_GROUP_FILTER_TAG,
		total,
	))
}

/// Searches signature groups by name, ordered by name.
///
/// Case-insensitive substring match on the signature group name. Returns an empty array when nothing matches.
#[utoipa::path(
	get,
	tag = "Signature Group",
	params(SignatureGroupSearchQuery, PageParams),
	responses(
		(status = 200, description = "One page of matching signature groups ordered by name", body = PageOfPlaymatchSignatureGroup),
		(status = 400, description = "Empty search query, or a malformed pagination cursor or filter-set mismatch", body = V2ErrorBody)
	)
)]
#[get("/signature-groups/search")]
pub async fn search_signature_groups_v2(
	query: Query<SignatureGroupSearchQuery>,
	params: Query<PageParams>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let query = query.into_inner();
	let literal = resolve_or_return!(require_non_empty_query(&query.query));
	let params = params.into_inner();
	let limit = params.limit_clamped();
	let filter_tag = search_filter_tag(literal);
	let start = resolve_or_return!(resolve_keyset_start(&params, filter_tag));
	let has_previous = start.has_previous();

	let page = search_signature_groups_by_name_page(
		literal,
		start.after(),
		Some(limit),
		db_conn.get_ref(),
	)
	.await?;
	let page = page.map_rows(PlaymatchSignatureGroupV2::from);

	Ok(build_page(
		page.rows,
		page.has_more,
		has_previous,
		limit,
		filter_tag,
		None,
	))
}

/// Returns a signature group by id.
#[utoipa::path(
	get,
	tag = "Signature Group",
	responses(
		(status = 200, description = "The signature group", body = PlaymatchSignatureGroupV2),
		(status = 404, description = "Signature group not found", body = V2ErrorBody)
	)
)]
#[get("/signature-groups/{id}")]
pub async fn get_signature_group_by_id_v2(
	id: Path<Uuid>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let signature_group = find_signature_group_by_id(id.into_inner(), db_conn.get_ref())
		.await?
		.map(PlaymatchSignatureGroupV2::from);

	Ok(ok_or_v2_not_found(
		signature_group,
		"signature_group_not_found",
		"signature group not found",
	))
}

/// Lists the dat files published under a signature group, ordered by name.
///
/// Pass `platform_id` to narrow the listing to a single platform. If omitted, all platforms are included.
#[utoipa::path(
	get,
	tag = "Signature Group",
	params(SignatureGroupDatFilesQuery, PageParams),
	responses(
		(status = 200, description = "One page of the signature group's dat files ordered by name", body = PageOfDatFileSummary),
		(status = 400, description = "Malformed pagination cursor or filter-set mismatch", body = V2ErrorBody),
		(status = 404, description = "Signature group not found", body = V2ErrorBody)
	)
)]
#[get("/signature-groups/{id}/dat-files")]
pub async fn list_signature_group_dat_files_v2(
	id: Path<Uuid>,
	query: Query<SignatureGroupDatFilesQuery>,
	params: Query<PageParams>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let signature_group_id = id.into_inner();
	let platform_id = query.into_inner().platform_id;
	let params = params.into_inner();
	let limit = params.limit_clamped();
	let filter_tag = dat_files_filter_tag(signature_group_id, platform_id);
	let start = resolve_or_return!(resolve_keyset_start(&params, filter_tag));
	let has_previous = start.has_previous();

	let page = find_signature_group_dat_files_page(
		signature_group_id,
		platform_id,
		start.after(),
		Some(limit),
		db_conn.get_ref(),
	)
	.await?;

	match page {
		// dat_file is write-hot, so with_total is ignored here by design.
		Some(page) => Ok(build_page(
			page.rows,
			page.has_more,
			has_previous,
			limit,
			filter_tag,
			None,
		)),
		None => Ok(v2_not_found(
			"signature_group_not_found",
			"signature group not found",
		)),
	}
}

/// Lists the games across every dat file published under a signature group, ordered by name.
///
/// Pass `platform_id` to narrow the listing to a single platform. If omitted, all platforms are included. Defaults to current games only. Clones are excluded.
#[utoipa::path(
	get,
	tag = "Signature Group",
	params(SignatureGroupGamesQuery, PageParams),
	responses(
		(status = 200, description = "One page of the signature group's games ordered by name", body = PageOfGameMetadataResponse),
		(status = 400, description = "Malformed pagination cursor or filter-set mismatch", body = V2ErrorBody),
		(status = 404, description = "Signature group not found", body = V2ErrorBody)
	)
)]
#[get("/signature-groups/{id}/games")]
pub async fn list_signature_group_games_v2(
	id: Path<Uuid>,
	query: Query<SignatureGroupGamesQuery>,
	params: Query<PageParams>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let signature_group_id = id.into_inner();
	let query = query.into_inner();
	let params = params.into_inner();
	let limit = params.limit_clamped();
	let current_only = query.current_only.unwrap_or(true);
	let filter_tag = games_filter_tag(signature_group_id, query.platform_id, current_only);
	let start = resolve_or_return!(resolve_keyset_start(&params, filter_tag));
	let has_previous = start.has_previous();

	let page = find_signature_group_games_page(
		signature_group_id,
		query.platform_id,
		current_only,
		start.after(),
		Some(limit),
		db_conn.get_ref(),
	)
	.await?;

	match page {
		// games is write-hot, so with_total is ignored here by design.
		Some(page) => Ok(build_page(
			page.rows,
			page.has_more,
			has_previous,
			limit,
			filter_tag,
			None,
		)),
		None => Ok(v2_not_found(
			"signature_group_not_found",
			"signature group not found",
		)),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn dat_files_tag_is_stable_and_separates_group_and_platform() {
		let group = Uuid::new_v4();
		let other = Uuid::new_v4();
		let platform = Uuid::new_v4();

		assert_eq!(
			dat_files_filter_tag(group, Some(platform)),
			dat_files_filter_tag(group, Some(platform))
		);
		assert_ne!(
			dat_files_filter_tag(group, None),
			dat_files_filter_tag(group, Some(platform))
		);
		assert_ne!(
			dat_files_filter_tag(group, Some(platform)),
			dat_files_filter_tag(other, Some(platform))
		);
	}

	#[test]
	fn dat_files_tag_does_not_collide_with_group_list_tag() {
		assert_ne!(
			dat_files_filter_tag(Uuid::nil(), None),
			SIGNATURE_GROUP_FILTER_TAG
		);
	}

	#[test]
	fn games_tag_is_stable_and_separates_group_platform_and_current_flag() {
		let group = Uuid::new_v4();
		let other = Uuid::new_v4();
		let platform = Uuid::new_v4();

		assert_eq!(
			games_filter_tag(group, Some(platform), true),
			games_filter_tag(group, Some(platform), true)
		);
		assert_ne!(
			games_filter_tag(group, Some(platform), true),
			games_filter_tag(group, Some(platform), false)
		);
		assert_ne!(
			games_filter_tag(group, None, true),
			games_filter_tag(group, Some(platform), true)
		);
		assert_ne!(
			games_filter_tag(group, Some(platform), true),
			games_filter_tag(other, Some(platform), true)
		);
	}

	#[test]
	fn games_tag_does_not_collide_with_dat_files_or_group_list_tags() {
		let group = Uuid::new_v4();
		assert_ne!(
			games_filter_tag(group, None, true),
			dat_files_filter_tag(group, None)
		);
		assert_ne!(
			games_filter_tag(Uuid::nil(), None, true),
			SIGNATURE_GROUP_FILTER_TAG
		);
	}
}
