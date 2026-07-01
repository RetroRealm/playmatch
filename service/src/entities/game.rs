use crate::db::game::{
	GAME_NAME_SEARCH_DEFAULT_LIMIT, GameBrowseFilters, dat_file_has_unresolved_clones,
	find_all_children_of_game, find_all_signature_metadata_mappings_for_game, find_game_parent,
	find_games_browse_page, find_games_by_name_ci_and_platform_id, get_dat_file_id_of_game,
	get_game_by_id, get_game_file_presence_observations, search_games_by_name,
	search_games_by_name_page,
};
use crate::db::game_file::{find_game_files_page, get_game_file_by_id};
use crate::db::pagination::KeysetPage;
use crate::db::signature_metadata_mapping::find_signature_metadata_mappings_by_game_ids;
use crate::model::{
	AutomaticMatchReasonV2, GameMetadataResponse, GameNameSearchResult, MetadataMatchType,
	MetadataProvider, PlaymatchGameFile,
};
use chrono::{DateTime, Utc};
use entity::game;
use sea_orm::DbConn;
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub async fn search_games_by_name_and_platform(
	query: &str,
	platform_id: Option<Uuid>,
	limit: Option<u64>,
	db_conn: &DbConn,
) -> anyhow::Result<Vec<GameNameSearchResult>> {
	let limit = limit.unwrap_or(GAME_NAME_SEARCH_DEFAULT_LIMIT);
	let rows = search_games_by_name(query, platform_id, limit, db_conn).await?;

	Ok(rows
		.into_iter()
		.map(
			|(id, name, platform_id, platform_name)| GameNameSearchResult {
				id,
				name,
				platform_id,
				platform_name,
			},
		)
		.collect())
}

/// One keyset page of browsed games ordered by `(name, id)`, each mapped to the
/// public DTO with its external metadata. `after` is the last row of the
/// previous page.
pub async fn find_games_browse_page_and_metadata(
	filters: &GameBrowseFilters,
	after: Option<(String, Uuid)>,
	limit: Option<u64>,
	db_conn: &DbConn,
) -> anyhow::Result<KeysetPage<GameMetadataResponse>> {
	let page = find_games_browse_page(filters, after, limit, db_conn).await?;

	let game_ids: Vec<Uuid> = page.rows.iter().map(|game| game.id).collect();
	let mut mappings_by_game =
		find_signature_metadata_mappings_by_game_ids(&game_ids, db_conn).await?;

	let mut rows = Vec::with_capacity(page.rows.len());
	for game in page.rows {
		let mappings = mappings_by_game.remove(&game.id).unwrap_or_default();
		rows.push(GameMetadataResponse {
			id: game.id,
			name: game.name,
			description: game.description,
			categories: game.categories,
			clone_of: game.clone_of,
			created_at: game.created_at.into(),
			updated_at: game.updated_at.into(),
			external_metadata: mappings.into_iter().map(Into::into).collect(),
		});
	}

	Ok(KeysetPage {
		rows,
		has_more: page.has_more,
	})
}

/// A single fuzzy-search candidate paired with its similarity score, used to
/// mint the `(score, id)` keyset cursor for the paginated v2 search.
pub struct ScoredGameSearchResult {
	pub score: f64,
	pub result: GameNameSearchResult,
}

/// One keyset page of a game's ROM file records ordered by `(file_name, id)`.
/// Returns `None` when the game does not exist so the caller can answer 404.
pub async fn find_game_files_page_for_game(
	game_id: Uuid,
	current_only: bool,
	after: Option<(String, Uuid)>,
	limit: Option<u64>,
	db_conn: &DbConn,
) -> anyhow::Result<Option<KeysetPage<PlaymatchGameFile>>> {
	if get_game_by_id(game_id, db_conn).await?.is_none() {
		return Ok(None);
	}

	let page = find_game_files_page(game_id, current_only, after, limit, db_conn).await?;
	Ok(Some(page.map_rows(Into::into)))
}

/// The public-safe projection of a game's provider mapping. Deliberately omits
/// `manually_matched_by` and `cross_match_last_tried_at`, which are internal
/// bookkeeping that must never reach an unauthenticated caller.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameProviderMapping {
	pub provider: MetadataProvider,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub provider_id: Option<String>,
	pub match_type: MetadataMatchType,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub automatic_match_reason: Option<AutomaticMatchReasonV2>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub matched_name: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub matched_year: Option<i16>,
}

/// Public provider mappings for a game, optionally narrowed to a single provider
/// and/or match type. Returns `None` when the game does not exist.
pub async fn find_game_provider_mappings(
	game_id: Uuid,
	provider: Option<MetadataProvider>,
	match_type: Option<MetadataMatchType>,
	db_conn: &DbConn,
) -> anyhow::Result<Option<Vec<GameProviderMapping>>> {
	if get_game_by_id(game_id, db_conn).await?.is_none() {
		return Ok(None);
	}

	let mappings = find_all_signature_metadata_mappings_for_game(game_id, db_conn).await?;
	let rows = mappings
		.into_iter()
		.map(|m| GameProviderMapping {
			provider: m.provider.into(),
			provider_id: m.provider_id,
			match_type: m.match_type.into(),
			automatic_match_reason: m.automatic_match_reason.map(Into::into),
			matched_name: m.matched_name,
			matched_year: m.matched_year,
		})
		.filter(|m| provider.is_none_or(|p| m.provider == p))
		.filter(|m| match_type.is_none_or(|t| m.match_type == t))
		.collect();

	Ok(Some(rows))
}

/// Which slice of the parent/clone graph a caller wants around a game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloneDirection {
	Children,
	Parent,
	Siblings,
	Tree,
}

/// A node in a game's parent/clone graph: just enough to follow up by id.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CloneGraphNode {
	pub id: Uuid,
	pub name: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub clone_of: Option<Uuid>,
}

impl From<game::Model> for CloneGraphNode {
	fn from(value: game::Model) -> Self {
		CloneGraphNode {
			id: value.id,
			name: value.name,
			clone_of: value.clone_of,
		}
	}
}

/// The parent/clone graph around a game. `parent` is the game this one clones;
/// `children` are the games that clone the requested game (for `tree`, the
/// children of the resolved parent). Returns `None` when the game does not exist.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CloneGraph {
	pub game: CloneGraphNode,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub parent: Option<CloneGraphNode>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub children: Vec<CloneGraphNode>,

	/// `false` when this game's dat file still has clone rows that are not linked to
	/// a resolved parent. That happens transiently right after an import while the
	/// deferred clone-resolution pass catches up, and permanently when a declared
	/// parent is absent from the dat file and so can never be linked. Either way the
	/// graph may be missing edges, so a caller relying on completeness (for example
	/// 1G1R) should treat an absent parent conservatively; retrying resolves the
	/// transient case but not the unresolvable one.
	pub clone_graph_complete: bool,
}

pub async fn find_game_clone_graph(
	game_id: Uuid,
	direction: CloneDirection,
	db_conn: &DbConn,
) -> anyhow::Result<Option<CloneGraph>> {
	let Some(game) = get_game_by_id(game_id, db_conn).await? else {
		return Ok(None);
	};

	let (parent, children) = match direction {
		CloneDirection::Children => (None, find_all_children_of_game(&game, db_conn).await?),
		CloneDirection::Parent => (find_game_parent(&game, db_conn).await?, Vec::new()),
		CloneDirection::Siblings => match find_game_parent(&game, db_conn).await? {
			Some(parent) => {
				let siblings = find_all_children_of_game(&parent, db_conn)
					.await?
					.into_iter()
					.filter(|sibling| sibling.id != game.id)
					.collect();
				(Some(parent), siblings)
			}
			None => (None, Vec::new()),
		},
		CloneDirection::Tree => match find_game_parent(&game, db_conn).await? {
			Some(parent) => {
				let children = find_all_children_of_game(&parent, db_conn).await?;
				(Some(parent), children)
			}
			None => (None, find_all_children_of_game(&game, db_conn).await?),
		},
	};

	// The clone graph is resolved from the `clone_of` column, which a deferred
	// pass populates per dat file after each import. If this game's dat file still
	// has unresolved rows, the graph may be missing edges; report that honestly.
	let dat_file_id = get_dat_file_id_of_game(&game, db_conn).await?;
	let clone_graph_complete = !dat_file_has_unresolved_clones(dat_file_id, db_conn).await?;

	Ok(Some(CloneGraph {
		game: game.into(),
		parent: parent.map(Into::into),
		children: children.into_iter().map(Into::into).collect(),
		clone_graph_complete,
	}))
}

/// Exact case-insensitive game lookup scoped to a platform. Deterministic, not
/// fuzzy; returns every game whose name matches exactly under that platform.
pub async fn find_games_by_exact_name_and_platform(
	name: &str,
	platform_id: Uuid,
	db_conn: &DbConn,
) -> anyhow::Result<Vec<GameMetadataResponse>> {
	let games = find_games_by_name_ci_and_platform_id(name, platform_id, db_conn).await?;

	let game_ids: Vec<Uuid> = games.iter().map(|game| game.id).collect();
	let mut mappings_by_game =
		find_signature_metadata_mappings_by_game_ids(&game_ids, db_conn).await?;

	let mut rows = Vec::with_capacity(games.len());
	for game in games {
		let mappings = mappings_by_game.remove(&game.id).unwrap_or_default();
		rows.push(GameMetadataResponse {
			id: game.id,
			name: game.name,
			description: game.description,
			categories: game.categories,
			clone_of: game.clone_of,
			created_at: game.created_at.into(),
			updated_at: game.updated_at.into(),
			external_metadata: mappings.into_iter().map(Into::into).collect(),
		});
	}

	Ok(rows)
}

/// One observation of a game file's hash inside a single dat file import. The
/// richer audit form behind the v2 presence changelog: where the v1 history
/// collapses to the list of imports, this keeps each presence row with its
/// import and the dat file it belongs to.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameFilePresenceEntry {
	pub presence_id: Uuid,
	pub observed_at: DateTime<Utc>,
	pub dat_file_id: Uuid,
	pub dat_file_name: String,
	pub signature_group_id: Uuid,
	pub platform_id: Uuid,
	pub dat_file_import_id: Uuid,
	pub import_name: String,
	pub version: String,
	pub imported_at: DateTime<Utc>,
}

/// The full per-import observation log for a game file's hash, newest import
/// first. Returns `None` when the game file does not exist so the caller can
/// answer 404.
pub async fn get_game_file_presence_changelog(
	game_file_id: Uuid,
	db_conn: &DbConn,
) -> anyhow::Result<Option<Vec<GameFilePresenceEntry>>> {
	if get_game_file_by_id(game_file_id, db_conn).await?.is_none() {
		return Ok(None);
	}

	let observations = get_game_file_presence_observations(game_file_id, db_conn).await?;
	let entries = observations
		.into_iter()
		.map(|(presence, import, dat_file)| GameFilePresenceEntry {
			presence_id: presence.id,
			observed_at: presence.created_at.into(),
			dat_file_id: dat_file.id,
			dat_file_name: dat_file.name,
			signature_group_id: dat_file.signature_group_id,
			platform_id: dat_file.platform_id,
			dat_file_import_id: import.id,
			import_name: import.name,
			version: import.version,
			imported_at: import.imported_at.into(),
		})
		.collect();

	Ok(Some(entries))
}

/// One keyset page of the fuzzy game-name search ordered by `(score DESC, id)`.
/// `after` is the `(score, id)` of the previous page's last row.
pub async fn search_games_by_name_page_and_platform(
	query: &str,
	platform_id: Option<Uuid>,
	after: Option<(f64, Uuid)>,
	limit: u64,
	db_conn: &DbConn,
) -> anyhow::Result<Vec<ScoredGameSearchResult>> {
	let rows = search_games_by_name_page(query, platform_id, after, limit, db_conn).await?;

	Ok(rows
		.into_iter()
		.map(
			|(score, id, name, platform_id, platform_name)| ScoredGameSearchResult {
				score,
				result: GameNameSearchResult {
					id,
					name,
					platform_id,
					platform_name,
				},
			},
		)
		.collect())
}
