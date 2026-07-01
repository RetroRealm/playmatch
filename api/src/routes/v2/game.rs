use crate::error;
use crate::model::game::{
	CloneDirectionParam, GameBrowseQuery, GameByNameQuery, GameClonesQuery, GameFilesQuery,
	GameMappingsQuery, GameSearchPageQuery,
};
use crate::model::igdb::MAX_SEARCH_LITERAL_LEN;
use crate::model::pagination::{FNV_OFFSET, PageParams, fnv1a, fnv1a_uuid};
use crate::routes::v2::error::{
	game_ok_or_v2_not_found, ok_or_v2_not_found, v2_bad_request, v2_not_found,
};
use crate::routes::v2::{
	build_page, build_score_page, require_non_empty_query, resolve_keyset_start, resolve_or_return,
	resolve_score_keyset_start,
};
use actix_web::web::{Data, Path, Query};
use actix_web::{HttpResponse, Responder, get, web};
use sea_orm::DatabaseConnection;
use service::db::game::GameBrowseFilters;
use service::db::game_file::get_game_file_by_id;
use service::entities::game::{
	CloneDirection, find_game_clone_graph, find_game_files_page_for_game,
	find_game_provider_mappings, find_games_browse_page_and_metadata,
	find_games_by_exact_name_and_platform, get_game_file_presence_changelog,
	search_games_by_name_page_and_platform,
};
use service::identification::{
	get_game_and_all_relations, get_game_by_id_from_db, get_game_file_history,
};
use service::model::{
	GameAndRelationsResultV2, GameNameSearchResultV2, PlaymatchDatFileImportV2, PlaymatchGameFile,
	PlaymatchGameFileV2,
};
use uuid::Uuid;

/// Base discriminator for every games-browse cursor. The active filter set is
/// folded into this so a cursor minted under one filter set is rejected under
/// another.
const GAME_BROWSE_FILTER_TAG_BASE: u64 = 0x6761_6d65_6273_7700;

const GAME_SEARCH_FILTER_TAG_BASE: u64 = 0x6761_6d65_7372_6300;

const GAME_FILES_FILTER_TAG_BASE: u64 = 0x6761_6d65_6669_6c00;

fn browse_filters(query: &GameBrowseQuery) -> GameBrowseFilters {
	GameBrowseFilters {
		platform_id: query.platform_id,
		signature_group_id: query.signature_group_id,
		company_id: query.company_id,
		current_only: query.current_only.unwrap_or(true),
		include_clones: query.clones.unwrap_or(false),
	}
}

/// Folds the active filters into the cursor tag so a cursor stays valid only for the filter set it was minted under.
fn browse_filter_tag(filters: &GameBrowseFilters) -> u64 {
	let mut hash = fnv1a_uuid(FNV_OFFSET, filters.platform_id);
	hash = fnv1a_uuid(hash, filters.signature_group_id);
	hash = fnv1a_uuid(hash, filters.company_id);
	hash = fnv1a(
		hash,
		&[filters.current_only as u8, filters.include_clones as u8],
	);
	GAME_BROWSE_FILTER_TAG_BASE ^ hash
}

fn search_filter_tag(query: &str, platform_id: Option<Uuid>) -> u64 {
	let hash = fnv1a(FNV_OFFSET, query.as_bytes());
	GAME_SEARCH_FILTER_TAG_BASE ^ fnv1a_uuid(hash, platform_id)
}

fn files_filter_tag(game_id: Uuid, current_only: bool) -> u64 {
	let hash = fnv1a(FNV_OFFSET, game_id.as_bytes());
	GAME_FILES_FILTER_TAG_BASE ^ fnv1a(hash, &[current_only as u8])
}

impl From<CloneDirectionParam> for CloneDirection {
	fn from(value: CloneDirectionParam) -> Self {
		match value {
			CloneDirectionParam::Children => CloneDirection::Children,
			CloneDirectionParam::Parent => CloneDirection::Parent,
			CloneDirectionParam::Siblings => CloneDirection::Siblings,
			CloneDirectionParam::Tree => CloneDirection::Tree,
		}
	}
}

/// Lists games ordered by name.
///
/// Optional `platform_id`, `signature_group_id`, and `company_id` narrow the result. Defaults to current games only. Clones are excluded unless `clones=true`.
#[utoipa::path(
	get,
	tag = "Game",
	params(GameBrowseQuery, PageParams),
	responses(
		(status = 200, description = "One page of games ordered by name", body = PageOfGameMetadataResponse),
		(status = 400, description = "Malformed pagination cursor or filter-set mismatch", body = V2ErrorBody)
	)
)]
#[get("/games")]
pub async fn list_games_v2(
	query: Query<GameBrowseQuery>,
	params: Query<PageParams>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let params = params.into_inner();
	let limit = params.limit_clamped();
	let filters = browse_filters(&query);
	let filter_tag = browse_filter_tag(&filters);
	let start = resolve_or_return!(resolve_keyset_start(&params, filter_tag));
	let has_previous = start.has_previous();

	let page = find_games_browse_page_and_metadata(
		&filters,
		start.after(),
		Some(limit),
		db_conn.get_ref(),
	)
	.await?;

	// games is write-hot, so with_total is ignored here by design.
	Ok(build_page(
		page.rows,
		page.has_more,
		has_previous,
		limit,
		filter_tag,
		None,
	))
}

/// Searches games by name, ordered by relevance.
///
/// Fuzzy match on the game name, narrowed to a single platform when `platform_id` is set. Returns an empty array when nothing matches.
#[utoipa::path(
	get,
	tag = "Game",
	params(GameSearchPageQuery, PageParams),
	responses(
		(status = 200, description = "One page of matching games ordered by relevance", body = PageOfGameNameSearchResult),
		(status = 400, description = "Empty or too-long query, or a malformed pagination cursor or filter-set mismatch", body = V2ErrorBody)
	)
)]
#[get("/games/search")]
pub async fn search_games_v2(
	query: Query<GameSearchPageQuery>,
	params: Query<PageParams>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let query = query.into_inner();
	let literal = resolve_or_return!(require_non_empty_query(&query.query));
	if query.query.chars().count() > MAX_SEARCH_LITERAL_LEN {
		return Ok(v2_bad_request(
			"query_too_long",
			format!("search query must be at most {MAX_SEARCH_LITERAL_LEN} characters"),
		));
	}

	let params = params.into_inner();
	let limit = params.limit_clamped();
	let filter_tag = search_filter_tag(literal, query.platform_id);
	let start = resolve_or_return!(resolve_score_keyset_start(&params, filter_tag));
	let has_previous = start.has_previous();

	let mut scored = search_games_by_name_page_and_platform(
		literal,
		query.platform_id,
		start.after(),
		limit + 1,
		db_conn.get_ref(),
	)
	.await?;

	let has_more = scored.len() as u64 > limit;
	if has_more {
		scored.truncate(limit as usize);
	}

	let positions: Vec<(f64, Uuid)> = scored.iter().map(|s| (s.score, s.result.id)).collect();
	let rows: Vec<GameNameSearchResultV2> = scored.into_iter().map(|s| s.result.into()).collect();

	Ok(build_score_page(
		rows,
		&positions,
		has_more,
		has_previous,
		limit,
		filter_tag,
	))
}

/// Returns a game by id.
#[utoipa::path(
	get,
	tag = "Game",
	responses(
		(status = 200, description = "The matched game and its metadata", body = GameMetadataResponse),
		(status = 404, description = "Game not found", body = V2ErrorBody)
	)
)]
#[get("/games/{id}")]
pub async fn get_game_by_id_v2(
	id: web::Path<Uuid>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	game_ok_or_v2_not_found(get_game_by_id_from_db(id.into_inner(), db_conn.get_ref()).await)
}

/// Returns a game by id with all related metadata.
#[utoipa::path(
	get,
	tag = "Game",
	responses(
		(status = 200, description = "The matched game with all related metadata", body = GameAndRelationsResultV2),
		(status = 404, description = "Game not found", body = V2ErrorBody)
	)
)]
#[get("/games/{id}/with-relations")]
pub async fn get_game_with_relations_by_id_v2(
	id: web::Path<Uuid>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let relations = get_game_and_all_relations(id.into_inner(), db_conn.get_ref())
		.await
		.map(GameAndRelationsResultV2::from);
	game_ok_or_v2_not_found(relations)
}

/// Lists a game's ROM files ordered by file name.
///
/// Defaults to files present in the latest dat release. Set `current_only=false` to include files dropped from earlier releases.
#[utoipa::path(
	get,
	tag = "Game",
	params(GameFilesQuery, PageParams),
	responses(
		(status = 200, description = "One page of the game's files ordered by file name", body = PageOfPlaymatchGameFile),
		(status = 400, description = "Malformed pagination cursor or filter-set mismatch", body = V2ErrorBody),
		(status = 404, description = "Game not found", body = V2ErrorBody)
	)
)]
#[get("/games/{id}/files")]
pub async fn list_game_files_v2(
	id: Path<Uuid>,
	query: Query<GameFilesQuery>,
	params: Query<PageParams>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let game_id = id.into_inner();
	let params = params.into_inner();
	let limit = params.limit_clamped();
	let current_only = query.current_only.unwrap_or(true);
	let filter_tag = files_filter_tag(game_id, current_only);
	let start = resolve_or_return!(resolve_keyset_start(&params, filter_tag));
	let has_previous = start.has_previous();

	let page = find_game_files_page_for_game(
		game_id,
		current_only,
		start.after(),
		Some(limit),
		db_conn.get_ref(),
	)
	.await?;

	match page {
		Some(page) => {
			let page = page.map_rows(PlaymatchGameFileV2::from);
			Ok(build_page(
				page.rows,
				page.has_more,
				has_previous,
				limit,
				filter_tag,
				None,
			))
		}
		None => Ok(v2_not_found("game_not_found", "game not found")),
	}
}

/// Lists a game's external provider mappings.
///
/// Set `provider` to restrict to one metadata provider and `match_type` to restrict to one match type. If omitted, all mappings are returned.
#[utoipa::path(
	get,
	tag = "Game",
	params(GameMappingsQuery),
	responses(
		(status = 200, description = "The game's external provider mappings", body = Vec<GameProviderMapping>),
		(status = 404, description = "Game not found", body = V2ErrorBody)
	)
)]
#[get("/games/{id}/mappings")]
pub async fn list_game_mappings_v2(
	id: Path<Uuid>,
	query: Query<GameMappingsQuery>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let query = query.into_inner();
	let mappings = find_game_provider_mappings(
		id.into_inner(),
		query.provider,
		query.match_type,
		db_conn.get_ref(),
	)
	.await?;

	Ok(ok_or_v2_not_found(
		mappings,
		"game_not_found",
		"game not found",
	))
}

/// Returns the parent and clone graph around a game.
///
/// `direction` selects which slice to return: `children`, `parent`, `siblings`, or `tree` for the full graph. Defaults to `tree`. The response carries `cloneGraphComplete`: it is `false` when this game's dat file still has clone rows not linked to a resolved parent, either transiently while the background resolution pass catches up after an import or permanently when a declared parent is absent from the dat file. A client that needs a complete graph (for example 1G1R) should treat an absent parent conservatively.
#[utoipa::path(
	get,
	tag = "Game",
	params(GameClonesQuery),
	responses(
		(status = 200, description = "The requested slice of the game's clone graph", body = CloneGraph),
		(status = 404, description = "Game not found", body = V2ErrorBody)
	)
)]
#[get("/games/{id}/clones")]
pub async fn list_game_clones_v2(
	id: Path<Uuid>,
	query: Query<GameClonesQuery>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let direction = query.into_inner().direction.unwrap_or_default();
	let graph = find_game_clone_graph(id.into_inner(), direction.into(), db_conn.get_ref()).await?;

	Ok(ok_or_v2_not_found(
		graph,
		"game_not_found",
		"game not found",
	))
}

/// Looks up games by exact name within a platform.
///
/// Case-insensitive exact match on `name`, scoped to `platform_id`. Returns every game whose name matches, or an empty array when nothing matches.
#[utoipa::path(
	get,
	tag = "Game",
	params(GameByNameQuery),
	responses(
		(status = 200, description = "Games matching the exact name under the platform", body = Vec<GameMetadataResponse>)
	)
)]
#[get("/games/by-name")]
pub async fn get_games_by_name_v2(
	query: Query<GameByNameQuery>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let query = query.into_inner();
	let games = find_games_by_exact_name_and_platform(
		query.name.trim(),
		query.platform_id,
		db_conn.get_ref(),
	)
	.await?;

	Ok(HttpResponse::Ok().json(games))
}

/// Returns a game file by id.
///
/// Carries the same projection as the files listed under `/games/{id}/files`.
#[utoipa::path(
	get,
	tag = "Game",
	responses(
		(status = 200, description = "The game file", body = PlaymatchGameFileV2),
		(status = 404, description = "Game file not found", body = V2ErrorBody)
	)
)]
#[get("/game-files/{id}")]
pub async fn get_game_file_by_id_v2(
	id: Path<Uuid>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let file = get_game_file_by_id(id.into_inner(), db_conn.get_ref())
		.await?
		.map(|file| PlaymatchGameFileV2::from(PlaymatchGameFile::from(file)));

	Ok(ok_or_v2_not_found(
		file,
		"game_file_not_found",
		"game file not found",
	))
}

/// Lists every import a game file's hash was observed in, newest import first.
///
/// Each entry pairs one observation of the hash with the import and dat file it was seen in.
#[utoipa::path(
	get,
	tag = "Game",
	responses(
		(status = 200, description = "Every per-import observation of the hash, newest import first", body = Vec<GameFilePresenceEntry>),
		(status = 404, description = "Game file not found", body = V2ErrorBody)
	)
)]
#[get("/game-files/{id}/presence")]
pub async fn get_game_file_presence_v2(
	id: Path<Uuid>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let changelog = get_game_file_presence_changelog(id.into_inner(), db_conn.get_ref()).await?;

	Ok(ok_or_v2_not_found(
		changelog,
		"game_file_not_found",
		"game file not found",
	))
}

/// Lists the dat file imports a game file was seen in, newest first.
///
/// Each entry is a dat file release in which this hash appeared.
#[utoipa::path(
	get,
	tag = "Game",
	responses(
		(status = 200, description = "The dat file imports this hash was seen in, newest first", body = Vec<PlaymatchDatFileImportV2>)
	)
)]
#[get("/game-files/{id}/history")]
pub async fn get_game_file_history_by_id_v2(
	id: Path<Uuid>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let history = get_game_file_history(id.into_inner(), db_conn.get_ref()).await?;
	let history: Vec<PlaymatchDatFileImportV2> = history.into_iter().map(Into::into).collect();
	Ok(HttpResponse::Ok().json(history))
}

#[cfg(test)]
mod tests {
	use super::*;

	fn filters(
		current_only: bool,
		include_clones: bool,
		platform: Option<Uuid>,
	) -> GameBrowseFilters {
		GameBrowseFilters {
			platform_id: platform,
			signature_group_id: None,
			company_id: None,
			current_only,
			include_clones,
		}
	}

	#[test]
	fn browse_tag_is_stable_for_identical_filters() {
		let id = Uuid::new_v4();
		assert_eq!(
			browse_filter_tag(&filters(true, false, Some(id))),
			browse_filter_tag(&filters(true, false, Some(id)))
		);
	}

	#[test]
	fn browse_tag_changes_when_any_filter_changes() {
		let base = browse_filter_tag(&filters(true, false, None));
		assert_ne!(base, browse_filter_tag(&filters(false, false, None)));
		assert_ne!(base, browse_filter_tag(&filters(true, true, None)));
		assert_ne!(
			base,
			browse_filter_tag(&filters(true, false, Some(Uuid::new_v4())))
		);
	}

	#[test]
	fn browse_tag_distinguishes_swapped_uuid_filters() {
		let a = Uuid::new_v4();
		let b = Uuid::new_v4();
		let by_platform = GameBrowseFilters {
			platform_id: Some(a),
			signature_group_id: Some(b),
			company_id: None,
			current_only: true,
			include_clones: false,
		};
		let swapped = GameBrowseFilters {
			platform_id: Some(b),
			signature_group_id: Some(a),
			..by_platform.clone()
		};
		assert_ne!(browse_filter_tag(&by_platform), browse_filter_tag(&swapped));
	}

	#[test]
	fn search_tag_depends_on_query_and_platform() {
		let p = Uuid::new_v4();
		assert_eq!(
			search_filter_tag("zelda", Some(p)),
			search_filter_tag("zelda", Some(p))
		);
		assert_ne!(
			search_filter_tag("zelda", Some(p)),
			search_filter_tag("mario", Some(p))
		);
		assert_ne!(
			search_filter_tag("zelda", Some(p)),
			search_filter_tag("zelda", None)
		);
	}

	#[test]
	fn browse_and_search_tag_spaces_do_not_collide() {
		let empty = browse_filter_tag(&filters(true, false, None));
		assert_ne!(empty, search_filter_tag("", None));
	}

	#[test]
	fn files_tag_depends_on_game_and_current_flag() {
		let a = Uuid::new_v4();
		let b = Uuid::new_v4();
		assert_eq!(files_filter_tag(a, true), files_filter_tag(a, true));
		assert_ne!(files_filter_tag(a, true), files_filter_tag(a, false));
		assert_ne!(files_filter_tag(a, true), files_filter_tag(b, true));
	}

	#[test]
	fn files_tag_does_not_collide_with_browse_or_search_spaces() {
		let files = files_filter_tag(Uuid::nil(), true);
		assert_ne!(files, browse_filter_tag(&filters(true, false, None)));
		assert_ne!(files, search_filter_tag("", None));
	}

	#[test]
	fn clone_direction_param_maps_to_service_direction() {
		assert_eq!(
			CloneDirection::from(CloneDirectionParam::Children),
			CloneDirection::Children
		);
		assert_eq!(
			CloneDirection::from(CloneDirectionParam::Parent),
			CloneDirection::Parent
		);
		assert_eq!(
			CloneDirection::from(CloneDirectionParam::Siblings),
			CloneDirection::Siblings
		);
		assert_eq!(
			CloneDirection::from(CloneDirectionParam::Tree),
			CloneDirection::Tree
		);
		assert_eq!(CloneDirectionParam::default(), CloneDirectionParam::Tree);
	}
}
