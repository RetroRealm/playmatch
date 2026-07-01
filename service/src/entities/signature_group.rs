use crate::db::dat_file::DatFileBrowseFilters;
use crate::db::game::GameBrowseFilters;
use crate::db::pagination::KeysetPage;
use crate::entities::dat_file::{DatFileSummary, find_dat_files_page};
use crate::entities::game::find_games_browse_page_and_metadata;
use crate::model::{GameMetadataResponse, PlaymatchSignatureGroup};
use sea_orm::DbConn;
use sea_orm::prelude::Uuid;

pub async fn find_all_signature_groups(
	db_conn: &DbConn,
) -> anyhow::Result<Vec<PlaymatchSignatureGroup>> {
	let groups = crate::db::signature_group::find_all_signature_groups(db_conn).await?;
	Ok(groups.into_iter().map(Into::into).collect())
}

pub async fn find_signature_group_by_id(
	id: Uuid,
	db_conn: &DbConn,
) -> anyhow::Result<Option<PlaymatchSignatureGroup>> {
	let group = crate::db::signature_group::find_signature_group_by_id(id, db_conn).await?;
	Ok(group.map(Into::into))
}

/// One keyset page of signature groups ordered by `(name, id)`, mapped to the
/// public DTO. `after` is the last row of the previous page.
pub async fn find_signature_groups_page(
	after: Option<(String, Uuid)>,
	limit: Option<u64>,
	db_conn: &DbConn,
) -> anyhow::Result<KeysetPage<PlaymatchSignatureGroup>> {
	let page =
		crate::db::signature_group::find_signature_groups_page(after, limit, db_conn).await?;
	Ok(page.map_rows(Into::into))
}

pub async fn count_signature_groups(db_conn: &DbConn) -> anyhow::Result<u64> {
	Ok(crate::db::signature_group::count_signature_groups(db_conn).await?)
}

/// One keyset page of the dat files published under a signature group, ordered by
/// `(name, id)` and optionally narrowed to a single platform. Returns `None` when
/// the signature group does not exist so the caller can answer 404.
pub async fn find_signature_group_dat_files_page(
	signature_group_id: Uuid,
	platform_id: Option<Uuid>,
	after: Option<(String, Uuid)>,
	limit: Option<u64>,
	db_conn: &DbConn,
) -> anyhow::Result<Option<KeysetPage<DatFileSummary>>> {
	if find_signature_group_by_id(signature_group_id, db_conn)
		.await?
		.is_none()
	{
		return Ok(None);
	}

	let filters = DatFileBrowseFilters {
		signature_group_id: Some(signature_group_id),
		platform_id,
		..Default::default()
	};
	let page = find_dat_files_page(&filters, after, limit, db_conn).await?;
	Ok(Some(page))
}

/// One keyset page of games across every dat file published under a signature
/// group, ordered by `(name, id)`. Narrowed by optional platform and to current
/// games only by default. Clones are excluded to match the catalogue browse.
/// Returns `None` when the signature group does not exist so the caller can
/// answer 404.
pub async fn find_signature_group_games_page(
	signature_group_id: Uuid,
	platform_id: Option<Uuid>,
	current_only: bool,
	after: Option<(String, Uuid)>,
	limit: Option<u64>,
	db_conn: &DbConn,
) -> anyhow::Result<Option<KeysetPage<GameMetadataResponse>>> {
	if find_signature_group_by_id(signature_group_id, db_conn)
		.await?
		.is_none()
	{
		return Ok(None);
	}

	let filters = GameBrowseFilters {
		platform_id,
		signature_group_id: Some(signature_group_id),
		company_id: None,
		current_only,
		include_clones: false,
	};
	let page = find_games_browse_page_and_metadata(&filters, after, limit, db_conn).await?;
	Ok(Some(page))
}
