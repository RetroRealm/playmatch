use crate::db::company::find_companies_by_ids;
use crate::db::dat_file::{
	DatFileBrowseFilters, count_current_games_in_dat_file, count_games_in_dat_file,
	find_all_dat_files, find_games_in_dat_file_page, get_dat_file_by_id,
	get_latest_import_for_dat_file, get_latest_imports_for_dat_files,
};
use crate::db::dat_file_import::{find_imports_for_dat_file_page, get_dat_file_import_by_id};
use crate::db::game::{
	find_all_relations_of_game, get_game_file_presence_with_dat_files,
	get_game_file_presence_with_dat_files_for_files,
};
use crate::db::game_file::{
	find_game_file_by_crc, find_game_file_by_md5, find_game_file_by_sha1, find_game_file_by_sha256,
	get_game_files_from_game_ids,
};
use crate::db::pagination::KeysetPage;
use crate::db::platform::find_platforms_by_ids;
use crate::db::signature_group::find_signature_groups_by_ids;
use crate::db::signature_metadata_mapping::find_signature_metadata_mappings_by_game_ids;
use crate::model::{ExternalMetadata, PlaymatchGameFileV2};
use chrono::{DateTime, Utc};
use entity::{company, dat_file, dat_file_import, game, platform, signature_group};
use sea_orm::DbConn;
use sea_orm::EntityTrait;
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

/// One file-hash lookup against the v2 reverse-lookup endpoint. At least one hash
/// must be supplied. The strongest supplied hash wins (sha256 > sha1 > md5 > crc),
/// matching the identify cascade; when two hashes resolve to different files the
/// strongest hash is authoritative.
#[derive(Debug, Clone, Default)]
pub struct HashLookup {
	pub sha256: Option<String>,
	pub sha1: Option<String>,
	pub md5: Option<String>,
	pub crc: Option<String>,
}

impl HashLookup {
	pub fn is_empty(&self) -> bool {
		self.sha256.is_none() && self.sha1.is_none() && self.md5.is_none() && self.crc.is_none()
	}
}

/// A named reference (id + name) to a related entity embedded in a dat-file
/// projection. Keeps the catalogue response self-describing without a second
/// round trip to resolve ids.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NamedRef {
	pub id: Uuid,
	pub name: String,
}

/// The most recent import recorded for a dat file.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LatestDatFileImport {
	pub id: Uuid,
	pub version: String,
	pub imported_at: DateTime<Utc>,
}

impl From<dat_file_import::Model> for LatestDatFileImport {
	fn from(value: dat_file_import::Model) -> Self {
		LatestDatFileImport {
			id: value.id,
			version: value.version,
			imported_at: value.imported_at.into(),
		}
	}
}

/// A dat file with its related signature group, platform and optional company
/// resolved to names. Returned by the v2 dat-file list.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DatFileSummary {
	pub id: Uuid,
	pub name: String,
	pub signature_group: NamedRef,
	pub platform: NamedRef,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub company: Option<NamedRef>,
	pub current_version: String,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub tags: Vec<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub subset: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub latest_dat_file_import: Option<LatestDatFileImport>,
}

/// A dat file with the [`DatFileSummary`] fields plus aggregate game counts.
/// Returned by the v2 single dat-file read.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DatFileDetail {
	#[serde(flatten)]
	pub summary: DatFileSummary,
	pub game_count: u64,
	pub current_game_count: u64,
}

/// A game in a dat file, optionally hydrated with its files and external
/// metadata mappings. Hydration is opt-in to keep the default listing cheap.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DatFileGame {
	pub id: Uuid,
	pub name: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub categories: Option<Vec<String>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub clone_of: Option<Uuid>,
	pub current_in_latest_dat: bool,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub files: Option<Vec<PlaymatchGameFileV2>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub external_metadata: Option<Vec<ExternalMetadata>>,
}

/// What to eagerly load alongside each game in a dat-file game listing.
#[derive(Debug, Clone, Copy, Default)]
pub struct DatFileGameHydration {
	pub include_files: bool,
	pub include_mappings: bool,
}

async fn build_summary(model: dat_file::Model, conn: &DbConn) -> anyhow::Result<DatFileSummary> {
	let (signature_group, platform, company, latest_import) = tokio::try_join!(
		async {
			Ok::<_, anyhow::Error>(
				signature_group::Entity::find_by_id(model.signature_group_id)
					.one(conn)
					.await?,
			)
		},
		async {
			Ok::<_, anyhow::Error>(
				platform::Entity::find_by_id(model.platform_id)
					.one(conn)
					.await?,
			)
		},
		async {
			match model.company_id {
				Some(company_id) => {
					Ok::<_, anyhow::Error>(company::Entity::find_by_id(company_id).one(conn).await?)
				}
				None => Ok(None),
			}
		},
		async { Ok::<_, anyhow::Error>(get_latest_import_for_dat_file(model.id, conn).await?) },
	)?;

	let signature_group = signature_group.ok_or_else(|| {
		anyhow::anyhow!(
			"signature group {} missing for dat file {}",
			model.signature_group_id,
			model.id
		)
	})?;
	let platform = platform.ok_or_else(|| {
		anyhow::anyhow!(
			"platform {} missing for dat file {}",
			model.platform_id,
			model.id
		)
	})?;

	Ok(DatFileSummary {
		id: model.id,
		name: model.name,
		signature_group: NamedRef {
			id: signature_group.id,
			name: signature_group.name,
		},
		platform: NamedRef {
			id: platform.id,
			name: platform.name,
		},
		company: company.map(|c| NamedRef {
			id: c.id,
			name: c.name,
		}),
		current_version: model.current_version,
		tags: model.tags.unwrap_or_default(),
		subset: model.subset,
		latest_dat_file_import: latest_import.map(Into::into),
	})
}

/// One keyset page of dat files ordered by `(name, id)`, each mapped to a
/// [`DatFileSummary`]. `after` is the last row of the previous page. The related
/// signature groups, platforms, companies and latest imports are resolved with
/// one bulk query each instead of a query per row.
pub async fn find_dat_files_page(
	filters: &DatFileBrowseFilters,
	after: Option<(String, Uuid)>,
	limit: Option<u64>,
	conn: &DbConn,
) -> anyhow::Result<KeysetPage<DatFileSummary>> {
	let page = find_all_dat_files(filters, after, limit, conn).await?;

	let dat_file_ids: Vec<Uuid> = page.rows.iter().map(|model| model.id).collect();
	let signature_group_ids: Vec<Uuid> = page
		.rows
		.iter()
		.map(|model| model.signature_group_id)
		.collect();
	let platform_ids: Vec<Uuid> = page.rows.iter().map(|model| model.platform_id).collect();
	let company_ids: Vec<Uuid> = page
		.rows
		.iter()
		.filter_map(|model| model.company_id)
		.collect();

	let (signature_groups, platforms, companies, latest_imports) = tokio::try_join!(
		async {
			Ok::<_, anyhow::Error>(find_signature_groups_by_ids(&signature_group_ids, conn).await?)
		},
		async { Ok::<_, anyhow::Error>(find_platforms_by_ids(&platform_ids, conn).await?) },
		async { Ok::<_, anyhow::Error>(find_companies_by_ids(&company_ids, conn).await?) },
		async {
			Ok::<_, anyhow::Error>(get_latest_imports_for_dat_files(&dat_file_ids, conn).await?)
		},
	)?;

	let mut rows = Vec::with_capacity(page.rows.len());
	for model in page.rows {
		rows.push(summary_from_maps(
			model,
			&signature_groups,
			&platforms,
			&companies,
			&latest_imports,
		)?);
	}

	Ok(KeysetPage {
		rows,
		has_more: page.has_more,
	})
}

fn summary_from_maps(
	model: dat_file::Model,
	signature_groups: &HashMap<Uuid, signature_group::Model>,
	platforms: &HashMap<Uuid, platform::Model>,
	companies: &HashMap<Uuid, company::Model>,
	latest_imports: &HashMap<Uuid, dat_file_import::Model>,
) -> anyhow::Result<DatFileSummary> {
	let signature_group = signature_groups
		.get(&model.signature_group_id)
		.ok_or_else(|| {
			anyhow::anyhow!(
				"signature group {} missing for dat file {}",
				model.signature_group_id,
				model.id
			)
		})?;
	let platform = platforms.get(&model.platform_id).ok_or_else(|| {
		anyhow::anyhow!(
			"platform {} missing for dat file {}",
			model.platform_id,
			model.id
		)
	})?;
	let company = model.company_id.and_then(|id| companies.get(&id));
	let latest_import = latest_imports.get(&model.id).cloned();

	Ok(DatFileSummary {
		id: model.id,
		name: model.name,
		signature_group: NamedRef {
			id: signature_group.id,
			name: signature_group.name.clone(),
		},
		platform: NamedRef {
			id: platform.id,
			name: platform.name.clone(),
		},
		company: company.map(|c| NamedRef {
			id: c.id,
			name: c.name.clone(),
		}),
		current_version: model.current_version,
		tags: model.tags.unwrap_or_default(),
		subset: model.subset,
		latest_dat_file_import: latest_import.map(Into::into),
	})
}

/// Load a dat file by id as a [`DatFileDetail`], or `None` if it does not exist.
pub async fn find_dat_file_detail_by_id(
	id: Uuid,
	conn: &DbConn,
) -> anyhow::Result<Option<DatFileDetail>> {
	let Some(model) = get_dat_file_by_id(id, conn).await? else {
		return Ok(None);
	};

	let summary = build_summary(model, conn).await?;
	let (game_count, current_game_count) = tokio::try_join!(
		async { Ok::<_, anyhow::Error>(count_games_in_dat_file(id, conn).await?) },
		async { Ok::<_, anyhow::Error>(count_current_games_in_dat_file(id, conn).await?) },
	)?;

	Ok(Some(DatFileDetail {
		summary,
		game_count,
		current_game_count,
	}))
}

/// One keyset page of games in a dat file ordered by `(name, id)`, optionally
/// hydrated with files and metadata mappings. `current_only` restricts to games
/// present in the current release. Returns `None` when the dat file is unknown.
pub async fn find_dat_file_games_page(
	dat_file_id: Uuid,
	current_only: bool,
	hydration: DatFileGameHydration,
	after: Option<(String, Uuid)>,
	limit: Option<u64>,
	conn: &DbConn,
) -> anyhow::Result<Option<KeysetPage<DatFileGame>>> {
	if get_dat_file_by_id(dat_file_id, conn).await?.is_none() {
		return Ok(None);
	}

	let page = find_games_in_dat_file_page(dat_file_id, current_only, after, limit, conn).await?;

	let game_ids: Vec<Uuid> = page.rows.iter().map(|g| g.id).collect();

	let mut files_by_game: HashMap<Uuid, Vec<PlaymatchGameFileV2>> = HashMap::new();
	if hydration.include_files {
		for file in get_game_files_from_game_ids(&game_ids, conn).await? {
			files_by_game
				.entry(file.game_id)
				.or_default()
				.push(crate::model::PlaymatchGameFile::from(file).into());
		}
	}

	let mut mappings_by_game: HashMap<Uuid, Vec<ExternalMetadata>> = HashMap::new();
	if hydration.include_mappings {
		for (game_id, mappings) in
			find_signature_metadata_mappings_by_game_ids(&game_ids, conn).await?
		{
			mappings_by_game.insert(game_id, mappings.into_iter().map(Into::into).collect());
		}
	}

	let rows = page
		.rows
		.into_iter()
		.map(|game| {
			build_dat_file_game(game, &hydration, &mut files_by_game, &mut mappings_by_game)
		})
		.collect();

	Ok(Some(KeysetPage {
		rows,
		has_more: page.has_more,
	}))
}

fn build_dat_file_game(
	game: game::Model,
	hydration: &DatFileGameHydration,
	files_by_game: &mut HashMap<Uuid, Vec<PlaymatchGameFileV2>>,
	mappings_by_game: &mut HashMap<Uuid, Vec<ExternalMetadata>>,
) -> DatFileGame {
	let files = hydration
		.include_files
		.then(|| files_by_game.remove(&game.id).unwrap_or_default());
	let external_metadata = hydration
		.include_mappings
		.then(|| mappings_by_game.remove(&game.id).unwrap_or_default());

	DatFileGame {
		id: game.id,
		name: game.name,
		description: game.description,
		categories: game.categories,
		clone_of: game.clone_of,
		current_in_latest_dat: game.is_current,
		created_at: game.created_at.into(),
		updated_at: game.updated_at.into(),
		files,
		external_metadata,
	}
}

/// A single import a hash or game was observed in, projected for the reverse
/// lookups. Mirrors [`LatestDatFileImport`] but reused for first/last seen.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DatFileImportRef {
	pub id: Uuid,
	pub version: String,
	pub imported_at: DateTime<Utc>,
}

impl From<dat_file_import::Model> for DatFileImportRef {
	fn from(value: dat_file_import::Model) -> Self {
		DatFileImportRef {
			id: value.id,
			version: value.version,
			imported_at: value.imported_at.into(),
		}
	}
}

/// A dat file that contains a looked-up hash or game, with the imports it was
/// first and last seen in and whether it is still present in the dat file's
/// current release.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DatFilePresence {
	#[serde(flatten)]
	pub dat_file: DatFileSummary,
	pub first_seen_import: DatFileImportRef,
	pub last_seen_import: DatFileImportRef,
	pub is_current_in_latest: bool,
}

/// A signature group that publishes one or more dat files containing a looked-up
/// hash or game. Collapses [`DatFilePresence`] rows for the `level=group` view.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SignatureGroupPresence {
	pub signature_group: NamedRef,
	pub first_seen_import: DatFileImportRef,
	pub last_seen_import: DatFileImportRef,
	pub is_current_in_latest: bool,
}

/// The reverse-lookup result, either per dat file or collapsed to the publishing
/// signature groups.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum PresenceResult {
	DatFiles(Vec<DatFilePresence>),
	SignatureGroups(Vec<SignatureGroupPresence>),
}

/// Aggregate every (import, dat file) observation of a hash up to the dat-file
/// level. For each dat file: the oldest and newest import the hash was seen in,
/// and whether one of those imports is the dat file's current release. Ordered by
/// dat file name then id for a stable response.
async fn aggregate_dat_file_presence(
	observations: Vec<(dat_file_import::Model, dat_file::Model)>,
	conn: &DbConn,
) -> anyhow::Result<Vec<DatFilePresence>> {
	let mut imports_by_dat: HashMap<Uuid, Vec<dat_file_import::Model>> = HashMap::new();
	let mut models_by_dat: HashMap<Uuid, dat_file::Model> = HashMap::new();
	for (import, dat_file) in observations {
		models_by_dat.entry(dat_file.id).or_insert(dat_file);
		imports_by_dat
			.entry(import.dat_file_id)
			.or_default()
			.push(import);
	}

	let models: Vec<dat_file::Model> = models_by_dat.into_values().collect();
	let dat_file_ids: Vec<Uuid> = models.iter().map(|model| model.id).collect();
	let signature_group_ids: Vec<Uuid> = models
		.iter()
		.map(|model| model.signature_group_id)
		.collect();
	let platform_ids: Vec<Uuid> = models.iter().map(|model| model.platform_id).collect();
	let company_ids: Vec<Uuid> = models.iter().filter_map(|model| model.company_id).collect();

	let (signature_groups, platforms, companies, latest_imports) = tokio::try_join!(
		async {
			Ok::<_, anyhow::Error>(find_signature_groups_by_ids(&signature_group_ids, conn).await?)
		},
		async { Ok::<_, anyhow::Error>(find_platforms_by_ids(&platform_ids, conn).await?) },
		async { Ok::<_, anyhow::Error>(find_companies_by_ids(&company_ids, conn).await?) },
		async {
			Ok::<_, anyhow::Error>(get_latest_imports_for_dat_files(&dat_file_ids, conn).await?)
		},
	)?;

	let mut presences = Vec::with_capacity(models.len());
	for dat_file in models {
		let mut imports = imports_by_dat.remove(&dat_file.id).unwrap_or_default();
		imports.sort_by_key(|import| (import.imported_at, import.id));
		let (Some(first), Some(last)) = (imports.first().cloned(), imports.last().cloned()) else {
			continue;
		};

		let is_current_in_latest = dat_file
			.latest_dat_file_import_id
			.is_some_and(|latest| imports.iter().any(|import| import.id == latest));

		let summary = summary_from_maps(
			dat_file,
			&signature_groups,
			&platforms,
			&companies,
			&latest_imports,
		)?;
		presences.push(DatFilePresence {
			dat_file: summary,
			first_seen_import: first.into(),
			last_seen_import: last.into(),
			is_current_in_latest,
		});
	}

	presences.sort_by(|a, b| {
		a.dat_file
			.name
			.cmp(&b.dat_file.name)
			.then(a.dat_file.id.cmp(&b.dat_file.id))
	});
	Ok(presences)
}

/// Collapse per-dat-file presence into per-signature-group presence: earliest
/// first-seen, latest last-seen and `is_current_in_latest` if any member dat file
/// is current. Ordered by signature group name then id.
fn collapse_to_signature_groups(presences: Vec<DatFilePresence>) -> Vec<SignatureGroupPresence> {
	let mut by_group: HashMap<Uuid, SignatureGroupPresence> = HashMap::new();
	for presence in presences {
		let group = presence.dat_file.signature_group.clone();
		let entry = by_group
			.entry(group.id)
			.or_insert_with(|| SignatureGroupPresence {
				signature_group: group,
				first_seen_import: presence.first_seen_import.clone(),
				last_seen_import: presence.last_seen_import.clone(),
				is_current_in_latest: false,
			});

		if presence.first_seen_import.imported_at < entry.first_seen_import.imported_at {
			entry.first_seen_import = presence.first_seen_import.clone();
		}
		if presence.last_seen_import.imported_at > entry.last_seen_import.imported_at {
			entry.last_seen_import = presence.last_seen_import.clone();
		}
		entry.is_current_in_latest |= presence.is_current_in_latest;
	}

	let mut groups: Vec<SignatureGroupPresence> = by_group.into_values().collect();
	groups.sort_by(|a, b| {
		a.signature_group
			.name
			.cmp(&b.signature_group.name)
			.then(a.signature_group.id.cmp(&b.signature_group.id))
	});
	groups
}

/// Resolve the game file a hash lookup points at using the identify cascade
/// precedence: the strongest supplied hash that matches a file wins, and the
/// strongest hash is authoritative even if a weaker one resolves elsewhere.
pub async fn resolve_game_file_for_hash_lookup(
	lookup: &HashLookup,
	conn: &DbConn,
) -> Result<Option<entity::game_file::Model>, sea_orm::DbErr> {
	if let Some(sha256) = lookup.sha256.as_deref().filter(|s| !s.is_empty())
		&& let Some(file) = find_game_file_by_sha256(sha256, conn).await?
	{
		return Ok(Some(file));
	}
	if let Some(sha1) = lookup.sha1.as_deref().filter(|s| !s.is_empty())
		&& let Some(file) = find_game_file_by_sha1(sha1, conn).await?
	{
		return Ok(Some(file));
	}
	if let Some(md5) = lookup.md5.as_deref().filter(|s| !s.is_empty())
		&& let Some(file) = find_game_file_by_md5(md5, conn).await?
	{
		return Ok(Some(file));
	}
	if let Some(crc) = lookup.crc.as_deref().filter(|s| !s.is_empty())
		&& let Some(file) = find_game_file_by_crc(crc, conn).await?
	{
		return Ok(Some(file));
	}
	Ok(None)
}

/// The dat files (or, when `as_groups`, the signature groups) whose imports
/// contain the file the hash lookup resolves to. Returns `None` when no game file
/// matches any supplied hash.
pub async fn find_dat_file_presence_for_hash_lookup(
	lookup: &HashLookup,
	as_groups: bool,
	conn: &DbConn,
) -> anyhow::Result<Option<PresenceResult>> {
	let Some(game_file) = resolve_game_file_for_hash_lookup(lookup, conn).await? else {
		return Ok(None);
	};

	let observations = get_game_file_presence_with_dat_files(game_file.id, conn).await?;
	let presences = aggregate_dat_file_presence(observations, conn).await?;
	Ok(Some(project_presence(presences, as_groups)))
}

/// The dat files (or signature groups, when `as_groups`) that contain a game.
/// Every game file of the game is unioned, so a dat file appears once with its
/// widest first/last-seen window. Returns `None` when the game does not exist.
pub async fn find_dat_file_presence_for_game(
	game_id: Uuid,
	as_groups: bool,
	conn: &DbConn,
) -> anyhow::Result<Option<PresenceResult>> {
	let Some(game) = crate::db::game::get_game_by_id(game_id, conn).await? else {
		return Ok(None);
	};

	let (_, _, _, _, _, game_files) = find_all_relations_of_game(&game, conn).await?;

	let game_file_ids: Vec<Uuid> = game_files.iter().map(|file| file.id).collect();
	let observations =
		get_game_file_presence_with_dat_files_for_files(&game_file_ids, conn).await?;

	let presences = aggregate_dat_file_presence(observations, conn).await?;
	Ok(Some(project_presence(presences, as_groups)))
}

/// One entry in a dat file's import timeline. Carries the file name (which
/// typically encodes the build date) alongside version and import time. The raw
/// md5 dedup field is deliberately omitted from this public projection.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DatFileImportTimelineEntry {
	pub id: Uuid,
	pub name: String,
	pub version: String,
	pub imported_at: DateTime<Utc>,
}

impl From<dat_file_import::Model> for DatFileImportTimelineEntry {
	fn from(value: dat_file_import::Model) -> Self {
		DatFileImportTimelineEntry {
			id: value.id,
			name: value.name,
			version: value.version,
			imported_at: value.imported_at.into(),
		}
	}
}

/// One keyset page of a dat file's import timeline, newest import first, ordered
/// by `(imported_at, id)` descending. Returns `None` when the dat file is
/// unknown so the caller can answer 404 rather than an empty page.
pub async fn find_dat_file_imports_page(
	dat_file_id: Uuid,
	after: Option<(DateTime<Utc>, Uuid)>,
	limit: Option<u64>,
	conn: &DbConn,
) -> anyhow::Result<Option<KeysetPage<DatFileImportTimelineEntry>>> {
	if get_dat_file_by_id(dat_file_id, conn).await?.is_none() {
		return Ok(None);
	}

	let page = find_imports_for_dat_file_page(dat_file_id, after, limit, conn).await?;
	Ok(Some(page.map_rows(Into::into)))
}

/// Load a single import addressed by its parent dat file and its own id. Returns
/// `None` when either is unknown or the import does not belong to the dat file,
/// so a mismatched pair reads as not found rather than leaking another dat's
/// import.
pub async fn find_dat_file_import_detail(
	dat_file_id: Uuid,
	import_id: Uuid,
	conn: &DbConn,
) -> anyhow::Result<Option<DatFileImportTimelineEntry>> {
	let Some(import) = get_dat_file_import_by_id(import_id, conn).await? else {
		return Ok(None);
	};
	if import.dat_file_id != dat_file_id {
		return Ok(None);
	}
	Ok(Some(import.into()))
}

fn project_presence(presences: Vec<DatFilePresence>, as_groups: bool) -> PresenceResult {
	if as_groups {
		PresenceResult::SignatureGroups(collapse_to_signature_groups(presences))
	} else {
		PresenceResult::DatFiles(presences)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use chrono::TimeZone;

	fn import_ref(id: Uuid, version: &str, imported_at: DateTime<Utc>) -> DatFileImportRef {
		DatFileImportRef {
			id,
			version: version.to_string(),
			imported_at,
		}
	}

	fn named(id: Uuid, name: &str) -> NamedRef {
		NamedRef {
			id,
			name: name.to_string(),
		}
	}

	fn summary(id: Uuid, name: &str, group: NamedRef) -> DatFileSummary {
		DatFileSummary {
			id,
			name: name.to_string(),
			signature_group: group,
			platform: named(Uuid::new_v4(), "platform"),
			company: None,
			current_version: "v".to_string(),
			tags: Vec::new(),
			subset: None,
			latest_dat_file_import: None,
		}
	}

	fn ts(secs: i64) -> DateTime<Utc> {
		Utc.timestamp_opt(secs, 0).unwrap()
	}

	#[test]
	fn collapse_picks_widest_window_and_any_current() {
		let group = named(Uuid::new_v4(), "No-Intro");
		let dat_a = Uuid::new_v4();
		let dat_b = Uuid::new_v4();

		let presences = vec![
			DatFilePresence {
				dat_file: summary(dat_a, "A", group.clone()),
				first_seen_import: import_ref(Uuid::new_v4(), "1", ts(100)),
				last_seen_import: import_ref(Uuid::new_v4(), "2", ts(200)),
				is_current_in_latest: false,
			},
			DatFilePresence {
				dat_file: summary(dat_b, "B", group.clone()),
				first_seen_import: import_ref(Uuid::new_v4(), "3", ts(50)),
				last_seen_import: import_ref(Uuid::new_v4(), "4", ts(300)),
				is_current_in_latest: true,
			},
		];

		let groups = collapse_to_signature_groups(presences);
		assert_eq!(groups.len(), 1);
		let g = &groups[0];
		assert_eq!(g.signature_group.id, group.id);
		assert_eq!(g.first_seen_import.imported_at, ts(50));
		assert_eq!(g.last_seen_import.imported_at, ts(300));
		assert!(g.is_current_in_latest);
	}

	#[test]
	fn collapse_keeps_distinct_groups_sorted_by_name() {
		let zebra = named(Uuid::new_v4(), "Zebra");
		let alpha = named(Uuid::new_v4(), "Alpha");
		let dat_z = Uuid::new_v4();
		let dat_a = Uuid::new_v4();

		let presences = vec![
			DatFilePresence {
				dat_file: summary(dat_z, "Z", zebra.clone()),
				first_seen_import: import_ref(Uuid::new_v4(), "1", ts(100)),
				last_seen_import: import_ref(Uuid::new_v4(), "1", ts(100)),
				is_current_in_latest: false,
			},
			DatFilePresence {
				dat_file: summary(dat_a, "A", alpha.clone()),
				first_seen_import: import_ref(Uuid::new_v4(), "1", ts(100)),
				last_seen_import: import_ref(Uuid::new_v4(), "1", ts(100)),
				is_current_in_latest: false,
			},
		];

		let groups = collapse_to_signature_groups(presences);
		assert_eq!(groups.len(), 2);
		assert_eq!(groups[0].signature_group.name, "Alpha");
		assert_eq!(groups[1].signature_group.name, "Zebra");
	}

	#[test]
	fn hash_lookup_is_empty_when_all_absent() {
		assert!(HashLookup::default().is_empty());
		assert!(
			!HashLookup {
				crc: Some("abcd".into()),
				..Default::default()
			}
			.is_empty()
		);
	}
}
