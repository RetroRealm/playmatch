use crate::error;
use crate::model::dat_file::{
	DatFileBrowseQuery, DatFileByHashQuery, DatFileGamesQuery, GameDatFilesQuery,
};
use crate::model::pagination::{FNV_OFFSET, PageParams, fnv1a, fnv1a_uuid};
use crate::routes::v2::error::{ok_or_v2_not_found, v2_bad_request, v2_not_found};
use crate::routes::v2::{
	build_page, build_time_page, resolve_keyset_start, resolve_or_return, resolve_time_keyset_start,
};
use actix_web::web::{Data, Path, Query};
use actix_web::{Responder, get};
use sea_orm::DatabaseConnection;
use service::db::dat_file::DatFileBrowseFilters;
use service::entities::dat_file::{
	DatFileGameHydration, find_dat_file_detail_by_id, find_dat_file_games_page,
	find_dat_file_import_detail, find_dat_file_imports_page, find_dat_file_presence_for_game,
	find_dat_file_presence_for_hash_lookup, find_dat_files_page,
};
use uuid::Uuid;

const DAT_FILE_BROWSE_FILTER_TAG_BASE: u64 = 0x6461_745f_6669_6c00;
const DAT_FILE_GAMES_FILTER_TAG_BASE: u64 = 0x6461_745f_676d_7300;
const DAT_FILE_IMPORTS_FILTER_TAG_BASE: u64 = 0x6461_745f_696d_7000;

fn fnv1a_opt_str(seed: u64, value: Option<&str>) -> u64 {
	match value.filter(|s| !s.is_empty()) {
		Some(s) => fnv1a(seed ^ 2, s.as_bytes()),
		None => fnv1a(seed, &[0]),
	}
}

fn browse_filters(query: &DatFileBrowseQuery) -> DatFileBrowseFilters {
	DatFileBrowseFilters {
		signature_group_id: query.signature_group_id,
		platform_id: query.platform_id,
		company_id: query.company_id,
		subset: query.subset.clone(),
		tag: query.tag.clone(),
		name_contains: query.name.clone(),
	}
}

/// Folds the active filters into the cursor tag so a cursor stays valid only for the filter set it was minted under.
fn browse_filter_tag(filters: &DatFileBrowseFilters) -> u64 {
	let mut hash = fnv1a_uuid(FNV_OFFSET, filters.signature_group_id);
	hash = fnv1a_uuid(hash, filters.platform_id);
	hash = fnv1a_uuid(hash, filters.company_id);
	hash = fnv1a_opt_str(hash, filters.subset.as_deref());
	hash = fnv1a_opt_str(hash, filters.tag.as_deref());
	hash = fnv1a_opt_str(hash, filters.name_contains.as_deref());
	DAT_FILE_BROWSE_FILTER_TAG_BASE ^ hash
}

fn games_filter_tag(dat_file_id: Uuid, current_only: bool) -> u64 {
	let hash = fnv1a(FNV_OFFSET, dat_file_id.as_bytes());
	DAT_FILE_GAMES_FILTER_TAG_BASE ^ fnv1a(hash, &[current_only as u8])
}

fn imports_filter_tag(dat_file_id: Uuid) -> u64 {
	DAT_FILE_IMPORTS_FILTER_TAG_BASE ^ fnv1a(FNV_OFFSET, dat_file_id.as_bytes())
}

/// Lists dat files ordered by name.
///
/// Narrow the result with optional signature group, platform, company, subset, tag, and name-substring filters. Keep the filters identical across pages.
#[utoipa::path(
	get,
	tag = "DAT File",
	params(DatFileBrowseQuery, PageParams),
	responses(
		(status = 200, description = "One page of dat files ordered by name", body = PageOfDatFileSummary),
		(status = 400, description = "Malformed pagination cursor or filter-set mismatch", body = V2ErrorBody)
	)
)]
#[get("/dat-files")]
pub async fn list_dat_files_v2(
	query: Query<DatFileBrowseQuery>,
	params: Query<PageParams>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let params = params.into_inner();
	let limit = params.limit_clamped();
	let filters = browse_filters(&query);
	let filter_tag = browse_filter_tag(&filters);
	let start = resolve_or_return!(resolve_keyset_start(&params, filter_tag));
	let has_previous = start.has_previous();

	let page = find_dat_files_page(&filters, start.after(), Some(limit), db_conn.get_ref()).await?;

	// dat_file is write-hot, so with_total is ignored here by design.
	Ok(build_page(
		page.rows,
		page.has_more,
		has_previous,
		limit,
		filter_tag,
		None,
	))
}

/// Lists the dat files or signature groups that contain a file by its hashes.
///
/// Returns one entry per dat file, or one entry per signature group when `level=group`. The strongest supplied hash resolves the file: sha256, then sha1, then md5, then crc. Each entry shows the first and last import the file appeared in and whether it is in the dat file's current release.
#[utoipa::path(
	get,
	tag = "DAT File",
	params(DatFileByHashQuery),
	responses(
		(status = 200, description = "The dat files or signature groups that contain the matched file", body = PresenceResult),
		(status = 400, description = "No hash supplied, or a supplied hash is malformed", body = V2ErrorBody),
		(status = 404, description = "No file matches any supplied hash", body = V2ErrorBody)
	)
)]
#[get("/dat-files/by-hash")]
pub async fn list_dat_files_by_hash_v2(
	query: Query<DatFileByHashQuery>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let query = query.into_inner();
	if let Err(message) = query.validate() {
		return Ok(v2_bad_request("invalid_hash", message));
	}

	let as_groups = query.level.unwrap_or_default().as_groups();
	let result =
		find_dat_file_presence_for_hash_lookup(&query.to_lookup(), as_groups, db_conn.get_ref())
			.await?;

	Ok(ok_or_v2_not_found(
		result,
		"file_not_found",
		"no file matches any supplied hash",
	))
}

/// Returns a dat file by id.
///
/// Includes the dat file's related entities and its aggregate game counts.
#[utoipa::path(
	get,
	tag = "DAT File",
	responses(
		(status = 200, description = "The dat file with its related entities and game counts", body = DatFileDetail),
		(status = 404, description = "DAT file not found", body = V2ErrorBody)
	)
)]
#[get("/dat-files/{id}")]
pub async fn get_dat_file_by_id_v2(
	id: Path<Uuid>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let detail = find_dat_file_detail_by_id(id.into_inner(), db_conn.get_ref()).await?;

	Ok(ok_or_v2_not_found(
		detail,
		"dat_file_not_found",
		"DAT file not found",
	))
}

/// Lists the games in a dat file ordered by name.
///
/// Defaults to current games only. File and metadata hydration are off by default and enabled with `include_files` and `include_mappings`.
#[utoipa::path(
	get,
	tag = "DAT File",
	params(DatFileGamesQuery, PageParams),
	responses(
		(status = 200, description = "One page of games in the dat file ordered by name", body = PageOfDatFileGame),
		(status = 400, description = "Malformed pagination cursor or filter-set mismatch", body = V2ErrorBody),
		(status = 404, description = "DAT file not found", body = V2ErrorBody)
	)
)]
#[get("/dat-files/{id}/games")]
pub async fn list_dat_file_games_v2(
	id: Path<Uuid>,
	query: Query<DatFileGamesQuery>,
	params: Query<PageParams>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let dat_file_id = id.into_inner();
	let query = query.into_inner();
	let params = params.into_inner();
	let limit = params.limit_clamped();
	let current_only = query.current_only.unwrap_or(true);
	let filter_tag = games_filter_tag(dat_file_id, current_only);
	let start = resolve_or_return!(resolve_keyset_start(&params, filter_tag));
	let has_previous = start.has_previous();

	let hydration = DatFileGameHydration {
		include_files: query.include_files.unwrap_or(false),
		include_mappings: query.include_mappings.unwrap_or(false),
	};

	let page = find_dat_file_games_page(
		dat_file_id,
		current_only,
		hydration,
		start.after(),
		Some(limit),
		db_conn.get_ref(),
	)
	.await?;

	match page {
		Some(page) => Ok(build_page(
			page.rows,
			page.has_more,
			has_previous,
			limit,
			filter_tag,
			None,
		)),
		None => Ok(v2_not_found("dat_file_not_found", "DAT file not found")),
	}
}

/// Lists the imports of a dat file, newest first.
///
/// Each entry carries the import id, file name, version, and import time. The raw md5 dedup field is omitted.
#[utoipa::path(
	get,
	tag = "DAT File",
	params(PageParams),
	responses(
		(status = 200, description = "One page of the dat file's imports, newest first", body = PageOfDatFileImportTimelineEntry),
		(status = 400, description = "Malformed pagination cursor or filter-set mismatch", body = V2ErrorBody),
		(status = 404, description = "DAT file not found", body = V2ErrorBody)
	)
)]
#[get("/dat-files/{id}/imports")]
pub async fn list_dat_file_imports_v2(
	id: Path<Uuid>,
	params: Query<PageParams>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let dat_file_id = id.into_inner();
	let params = params.into_inner();
	let limit = params.limit_clamped();
	let filter_tag = imports_filter_tag(dat_file_id);
	let start = resolve_or_return!(resolve_time_keyset_start(&params, filter_tag));
	let has_previous = start.has_previous();

	let page =
		find_dat_file_imports_page(dat_file_id, start.after(), Some(limit), db_conn.get_ref())
			.await?;

	match page {
		Some(page) => {
			let positions: Vec<_> = page.rows.iter().map(|i| (i.imported_at, i.id)).collect();
			Ok(build_time_page(
				page.rows,
				&positions,
				page.has_more,
				has_previous,
				limit,
				filter_tag,
			))
		}
		None => Ok(v2_not_found("dat_file_not_found", "DAT file not found")),
	}
}

/// Returns a single dat file import by id.
///
/// The import is addressed by its parent dat file id and its own import id.
#[utoipa::path(
	get,
	tag = "DAT File",
	responses(
		(status = 200, description = "The dat file import", body = DatFileImportTimelineEntry),
		(status = 404, description = "DAT file or import not found, or the import belongs to another dat file", body = V2ErrorBody)
	)
)]
#[get("/dat-files/{id}/imports/{importId}")]
pub async fn get_dat_file_import_v2(
	path: Path<(Uuid, Uuid)>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let (dat_id, import_id) = path.into_inner();
	let import = find_dat_file_import_detail(dat_id, import_id, db_conn.get_ref()).await?;

	Ok(ok_or_v2_not_found(
		import,
		"import_not_found",
		"DAT file import not found",
	))
}

/// Lists the dat files or signature groups that contain a game.
///
/// Returns one entry per dat file, or one entry per signature group when `level=group`, unioned across all of the game's files. Each entry shows the first and last import the file appeared in and whether it is in the dat file's current release.
#[utoipa::path(
	get,
	tag = "Game",
	params(GameDatFilesQuery),
	responses(
		(status = 200, description = "The dat files or signature groups that contain the game", body = PresenceResult),
		(status = 404, description = "Game not found", body = V2ErrorBody)
	)
)]
#[get("/games/{id}/dat-files")]
pub async fn list_game_dat_files_v2(
	id: Path<Uuid>,
	query: Query<GameDatFilesQuery>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let as_groups = query.into_inner().level.unwrap_or_default().as_groups();
	let result =
		find_dat_file_presence_for_game(id.into_inner(), as_groups, db_conn.get_ref()).await?;

	Ok(ok_or_v2_not_found(
		result,
		"game_not_found",
		"game not found",
	))
}

#[cfg(test)]
mod tests {
	use super::*;

	fn filters(
		name: Option<&str>,
		tag: Option<&str>,
		platform: Option<Uuid>,
	) -> DatFileBrowseFilters {
		DatFileBrowseFilters {
			signature_group_id: None,
			platform_id: platform,
			company_id: None,
			subset: None,
			tag: tag.map(str::to_string),
			name_contains: name.map(str::to_string),
		}
	}

	#[test]
	fn browse_tag_is_stable_for_identical_filters() {
		let id = Uuid::new_v4();
		assert_eq!(
			browse_filter_tag(&filters(Some("redump"), None, Some(id))),
			browse_filter_tag(&filters(Some("redump"), None, Some(id)))
		);
	}

	#[test]
	fn browse_tag_changes_when_any_filter_changes() {
		let base = browse_filter_tag(&filters(None, None, None));
		assert_ne!(base, browse_filter_tag(&filters(Some("x"), None, None)));
		assert_ne!(base, browse_filter_tag(&filters(None, Some("t"), None)));
		assert_ne!(
			base,
			browse_filter_tag(&filters(None, None, Some(Uuid::new_v4())))
		);
	}

	#[test]
	fn empty_and_absent_string_filters_tag_alike() {
		assert_eq!(
			browse_filter_tag(&filters(None, None, None)),
			browse_filter_tag(&filters(Some(""), Some(""), None))
		);
	}

	#[test]
	fn games_tag_depends_on_dat_file_and_current_flag() {
		let a = Uuid::new_v4();
		let b = Uuid::new_v4();
		assert_eq!(games_filter_tag(a, true), games_filter_tag(a, true));
		assert_ne!(games_filter_tag(a, true), games_filter_tag(a, false));
		assert_ne!(games_filter_tag(a, true), games_filter_tag(b, true));
	}

	#[test]
	fn browse_and_games_tag_spaces_do_not_collide() {
		let browse = browse_filter_tag(&filters(None, None, None));
		assert_ne!(browse, games_filter_tag(Uuid::nil(), true));
	}

	#[test]
	fn imports_tag_depends_on_dat_file() {
		let a = Uuid::new_v4();
		let b = Uuid::new_v4();
		assert_eq!(imports_filter_tag(a), imports_filter_tag(a));
		assert_ne!(imports_filter_tag(a), imports_filter_tag(b));
	}

	#[test]
	fn imports_tag_space_does_not_collide_with_browse_or_games() {
		let id = Uuid::new_v4();
		assert_ne!(
			imports_filter_tag(id),
			browse_filter_tag(&filters(None, None, None))
		);
		assert_ne!(imports_filter_tag(id), games_filter_tag(id, true));
	}
}
