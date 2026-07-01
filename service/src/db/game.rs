use crate::db::abstraction::ColumnEqIgnoreCaseTrait;
use crate::db::signature_metadata_mapping::find_signature_metadata_mappings_by_game_ids;
use crate::ingestion::parser::model;
use ::entity::{
	game, game::Entity as Game, game_file, game_file::Entity as GameFile,
	signature_metadata_mapping,
};
use chrono::{Duration, NaiveDateTime, Utc};
use entity::sea_orm_active_enums::{FailedMatchReasonEnum, MatchTypeEnum, MetadataProviderEnum};
use entity::{company, dat_file, dat_file_import, game_file_presence, platform, signature_group};
use futures_util::future::BoxFuture;
use sea_orm::prelude::Uuid;
use sea_orm::sea_query::extension::postgres::PgExpr;
use sea_orm::sea_query::{Alias, Expr};
use sea_orm::{
	ActiveEnum, ActiveModelTrait, ActiveValue::Set, ColumnTrait, DbConn, DbErr, EntityTrait,
	JoinType, ModelTrait, QueryFilter, QueryOrder, QuerySelect, RelationTrait, TryIntoModel,
	sea_query::SimpleExpr,
};
use std::collections::HashMap;

pub async fn get_game_by_id(game_id: Uuid, conn: &DbConn) -> Result<Option<game::Model>, DbErr> {
	Game::find_by_id(game_id).one(conn).await
}

pub async fn find_all_relations_of_game(
	game: &game::Model,
	conn: &DbConn,
) -> Result<
	(
		dat_file_import::Model,
		dat_file::Model,
		signature_group::Model,
		platform::Model,
		Option<company::Model>,
		Vec<game_file::Model>,
	),
	DbErr,
> {
	let dat_file_import = game
		.find_related(dat_file_import::Entity)
		.one(conn)
		.await?
		.ok_or_else(|| DbErr::RecordNotFound("Dat file import not found".to_string()))?;

	let dat_file = dat_file_import
		.find_related(dat_file::Entity)
		.one(conn)
		.await?
		.ok_or_else(|| DbErr::RecordNotFound("Dat file not found".to_string()))?;

	let (signature_group, platform) = tokio::try_join!(
		async {
			dat_file
				.find_related(signature_group::Entity)
				.one(conn)
				.await?
				.ok_or_else(|| DbErr::RecordNotFound("Signature group not found".to_string()))
		},
		async {
			dat_file
				.find_related(platform::Entity)
				.one(conn)
				.await?
				.ok_or_else(|| DbErr::RecordNotFound("Platform not found".to_string()))
		},
	)?;

	let (company, game_files) = tokio::try_join!(
		platform.find_related(company::Entity).one(conn),
		game.find_related(game_file::Entity).all(conn),
	)?;

	Ok((
		dat_file_import,
		dat_file,
		signature_group,
		platform,
		company,
		game_files,
	))
}

/// The dat file imports this hash was observed in, newest first.
pub async fn get_game_file_presence_history(
	game_file_id: Uuid,
	conn: &DbConn,
) -> Result<Vec<dat_file_import::Model>, DbErr> {
	let rows = game_file_presence::Entity::find()
		.filter(game_file_presence::Column::GameFileId.eq(game_file_id))
		.find_also_related(dat_file_import::Entity)
		.all(conn)
		.await?;

	let mut imports: Vec<dat_file_import::Model> =
		rows.into_iter().filter_map(|(_, import)| import).collect();
	imports.sort_by_key(|import| std::cmp::Reverse(import.imported_at));
	Ok(imports)
}

/// Every presence observation of a hash, newest import first. Unlike
/// [`get_game_file_presence_history`] this keeps the `game_file_presence` row
/// itself (its id and observation timestamp) alongside the import and dat file,
/// so the v2 changelog can expose the full per-import audit trail rather than the
/// collapsed list of imports.
pub async fn get_game_file_presence_observations(
	game_file_id: Uuid,
	conn: &DbConn,
) -> Result<
	Vec<(
		game_file_presence::Model,
		dat_file_import::Model,
		dat_file::Model,
	)>,
	DbErr,
> {
	let rows = game_file_presence::Entity::find()
		.filter(game_file_presence::Column::GameFileId.eq(game_file_id))
		.find_also_related(dat_file_import::Entity)
		.all(conn)
		.await?;

	let observed: Vec<(game_file_presence::Model, dat_file_import::Model)> = rows
		.into_iter()
		.filter_map(|(presence, import)| import.map(|import| (presence, import)))
		.collect();

	let dat_files =
		find_dat_files_by_ids(observed.iter().map(|(_, import)| import.dat_file_id), conn).await?;

	let mut out = Vec::with_capacity(observed.len());
	for (presence, import) in observed {
		if let Some(dat_file) = dat_files.get(&import.dat_file_id) {
			out.push((presence, import, dat_file.clone()));
		}
	}
	out.sort_by_key(|(_, import, _)| std::cmp::Reverse(import.imported_at));
	Ok(out)
}

/// Resolve the given dat file ids into a map keyed by id with one `is_in` query.
/// Empty input short-circuits without touching the database.
async fn find_dat_files_by_ids(
	ids: impl IntoIterator<Item = Uuid>,
	conn: &DbConn,
) -> Result<HashMap<Uuid, dat_file::Model>, DbErr> {
	let ids: Vec<Uuid> = ids.into_iter().collect();
	if ids.is_empty() {
		return Ok(HashMap::new());
	}
	let rows = dat_file::Entity::find()
		.filter(dat_file::Column::Id.is_in(ids))
		.all(conn)
		.await?;
	Ok(rows.into_iter().map(|row| (row.id, row)).collect())
}

/// Returns each (import, dat file) the hash appeared in, newest first. Resolves
/// the presence rows and their imports in one query, then loads every referenced
/// dat file in a single `is_in` query rather than one lookup per import.
pub async fn get_game_file_presence_with_dat_files(
	game_file_id: Uuid,
	conn: &DbConn,
) -> Result<Vec<(dat_file_import::Model, dat_file::Model)>, DbErr> {
	let imports = game_file_presence::Entity::find()
		.filter(game_file_presence::Column::GameFileId.eq(game_file_id))
		.find_also_related(dat_file_import::Entity)
		.all(conn)
		.await?
		.into_iter()
		.filter_map(|(_, import)| import)
		.collect::<Vec<_>>();

	let dat_files =
		find_dat_files_by_ids(imports.iter().map(|import| import.dat_file_id), conn).await?;

	let mut out = Vec::with_capacity(imports.len());
	for import in imports {
		if let Some(dat_file) = dat_files.get(&import.dat_file_id) {
			out.push((import, dat_file.clone()));
		}
	}
	out.sort_by_key(|(import, _)| std::cmp::Reverse(import.imported_at));
	Ok(out)
}

/// Batched variant of [`get_game_file_presence_with_dat_files`] over many game
/// files. Returns every (import, dat file) any of the `game_file_ids` appeared in,
/// resolving the presence rows and their imports in one query and the referenced
/// dat files in a single `is_in` query. The union may repeat an import when two of
/// the game's files share it; the caller aggregates per dat file so that is
/// harmless. Newest import first, matching the single-file reader.
pub async fn get_game_file_presence_with_dat_files_for_files(
	game_file_ids: &[Uuid],
	conn: &DbConn,
) -> Result<Vec<(dat_file_import::Model, dat_file::Model)>, DbErr> {
	if game_file_ids.is_empty() {
		return Ok(Vec::new());
	}

	let imports = game_file_presence::Entity::find()
		.filter(game_file_presence::Column::GameFileId.is_in(game_file_ids.iter().copied()))
		.find_also_related(dat_file_import::Entity)
		.all(conn)
		.await?
		.into_iter()
		.filter_map(|(_, import)| import)
		.collect::<Vec<_>>();

	let dat_files =
		find_dat_files_by_ids(imports.iter().map(|import| import.dat_file_id), conn).await?;

	let mut out = Vec::with_capacity(imports.len());
	for import in imports {
		if let Some(dat_file) = dat_files.get(&import.dat_file_id) {
			out.push((import, dat_file.clone()));
		}
	}
	out.sort_by_key(|(import, _)| std::cmp::Reverse(import.imported_at));
	Ok(out)
}

pub async fn insert_game(
	dat_file_import_id: Uuid,
	game: model::Game,
	conn: &DbConn,
) -> Result<game::Model, DbErr> {
	let game = game::ActiveModel {
		dat_file_import_id: Set(dat_file_import_id),
		signature_group_internal_id: Set(game.id),
		signature_group_internal_clone_of_id: Set(game.cloneofid),
		name: Set(game.name),
		description: Set(game.description),
		categories: Set(game.category),
		..Default::default()
	};

	game.save(conn).await?.try_into_model()
}

pub async fn find_game_by_signature_group_internal_id_and_dat_file_id(
	signature_group_internal_id: String,
	dat_file_id: Uuid,
	conn: &DbConn,
) -> Result<Option<game::Model>, DbErr> {
	Game::find()
		.filter(game::Column::SignatureGroupInternalId.eq(signature_group_internal_id))
		.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
		.filter(dat_file_import::Column::DatFileId.eq(dat_file_id))
		.one(conn)
		.await
}

/// Look up a game by exact name, scoped to a single dat file. A merged dat file
/// can hold a live and a retired same-name row, so prefer the current one with a
/// stable id tiebreaker to keep repeated imports matching the same game.
pub async fn find_game_by_name_and_dat_file_id(
	name: &str,
	dat_file_id: Uuid,
	conn: &DbConn,
) -> Result<Option<game::Model>, DbErr> {
	Game::find()
		.filter(game::Column::Name.eq(name))
		.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
		.filter(dat_file_import::Column::DatFileId.eq(dat_file_id))
		.order_by_desc(game::Column::IsCurrent)
		.order_by_asc(game::Column::Id)
		.one(conn)
		.await
}

pub async fn find_game_and_id_mapping_by_game_id(
	game_id: Uuid,
	conn: &DbConn,
) -> Result<Option<(game::Model, Option<signature_metadata_mapping::Model>)>, DbErr> {
	let game = Game::find()
		.filter(game::Column::Id.eq(game_id))
		.one(conn)
		.await?;

	let mapping = signature_metadata_mapping::Entity::find()
		.filter(signature_metadata_mapping::Column::GameId.eq(game_id))
		.one(conn)
		.await?;

	if let Some(game) = game {
		Ok(Some((game, mapping)))
	} else {
		Ok(None)
	}
}

pub async fn find_game_and_id_mapping_by_md5(
	md5: &str,
	conn: &DbConn,
) -> Result<Option<(game::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	find_signature_metadata_mapping_if_exists_by_filter(
		game_file::Column::Md5.eq_ignore_case(md5),
		conn,
	)
	.await
}

pub async fn find_game_and_id_mapping_by_sha1(
	sha1: &str,
	conn: &DbConn,
) -> Result<Option<(game::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	find_signature_metadata_mapping_if_exists_by_filter(
		game_file::Column::Sha1.eq_ignore_case(sha1),
		conn,
	)
	.await
}

pub async fn find_game_and_id_mapping_by_sha256(
	sha256: &str,
	conn: &DbConn,
) -> Result<Option<(game::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	find_signature_metadata_mapping_if_exists_by_filter(
		game_file::Column::Sha256.eq_ignore_case(sha256),
		conn,
	)
	.await
}

/// Matches a game by case-insensitive CRC32 and exact size, with its metadata
/// mappings. CRC32 is only 32 bits, so a same-CRC/different-size collision is a
/// real possibility at collection scale; pinning the size keeps the rung from
/// returning a confidently wrong identity. Mirrors the name+size lookup.
pub async fn find_game_and_id_mapping_by_crc(
	crc: &str,
	size: i64,
	conn: &DbConn,
) -> Result<Option<(game::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	find_signature_metadata_mapping_if_exists_by_filter(
		game_file::Column::Crc
			.eq_ignore_case(crc)
			.and(game_file::Column::FileSizeInBytes.eq(size)),
		conn,
	)
	.await
}

/// Matches a game by case-insensitive file name and exact size, with its metadata mappings.
pub async fn find_game_and_id_mapping_by_name_and_size(
	name: &str,
	size: i64,
	conn: &DbConn,
) -> Result<Option<(game::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	find_signature_metadata_mapping_if_exists_by_filter(
		game_file::Column::FileName
			.eq_ignore_case(name)
			.and(game_file::Column::FileSizeInBytes.eq(size)),
		conn,
	)
	.await
}

pub async fn find_games_by_name_and_platform_id(
	name: &str,
	platform_id: Uuid,
	conn: &DbConn,
) -> Result<Vec<game::Model>, DbErr> {
	Game::find()
		.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
		.join(
			JoinType::InnerJoin,
			dat_file_import::Relation::DatFile.def(),
		)
		.join(JoinType::InnerJoin, dat_file::Relation::Platform.def())
		.filter(
			game::Column::Name
				.eq(name)
				.and(platform::Column::Id.eq(platform_id)),
		)
		.all(conn)
		.await
}

/// Exact case-insensitive name lookup scoped to a platform. Unlike
/// [`find_games_by_name_and_platform_id`] (case-sensitive `=`), this matches on
/// `lower(name)` so the public lookup is deterministic regardless of casing.
/// Ordered by `(name, id)` for a stable response.
pub async fn find_games_by_name_ci_and_platform_id(
	name: &str,
	platform_id: Uuid,
	conn: &DbConn,
) -> Result<Vec<game::Model>, DbErr> {
	Game::find()
		.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
		.join(
			JoinType::InnerJoin,
			dat_file_import::Relation::DatFile.def(),
		)
		.join(JoinType::InnerJoin, dat_file::Relation::Platform.def())
		.filter(
			game::Column::Name
				.eq_ignore_case(name)
				.and(platform::Column::Id.eq(platform_id)),
		)
		.order_by_asc(game::Column::Name)
		.order_by_asc(game::Column::Id)
		.all(conn)
		.await
}

pub const GAME_NAME_SEARCH_DEFAULT_LIMIT: u64 = 25;

pub const GAME_NAME_SEARCH_MAX_LIMIT: u64 = 50;

/// Fuzzy-search games by name: case-insensitive substring or pg_trgm word
/// similarity, ordered by similarity and capped at [`GAME_NAME_SEARCH_MAX_LIMIT`].
pub async fn search_games_by_name(
	query: &str,
	platform_id: Option<Uuid>,
	limit: u64,
	conn: &DbConn,
) -> Result<Vec<(Uuid, String, Uuid, String)>, DbErr> {
	let capped_limit = limit.clamp(1, GAME_NAME_SEARCH_MAX_LIMIT);
	let pattern = format!("%{query}%");

	let mut select = Game::find()
		.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
		.join(
			JoinType::InnerJoin,
			dat_file_import::Relation::DatFile.def(),
		)
		.join(JoinType::InnerJoin, dat_file::Relation::Platform.def())
		.filter(
			Expr::col((game::Entity, game::Column::Name))
				.ilike(pattern)
				.or(Expr::cust_with_values("game.name %> $1", [query])),
		);

	if let Some(platform_id) = platform_id {
		select = select.filter(platform::Column::Id.eq(platform_id));
	}

	select
		.select_only()
		.column(game::Column::Id)
		.column(game::Column::Name)
		.column_as(platform::Column::Id, "platform_id")
		.column_as(platform::Column::Name, "platform_name")
		.order_by_desc(Expr::cust_with_values(
			"word_similarity($1, game.name)",
			[query],
		))
		.order_by_asc(game::Column::Name)
		.order_by_asc(game::Column::Id)
		.limit(capped_limit)
		.into_tuple::<(Uuid, String, Uuid, String)>()
		.all(conn)
		.await
}

/// Filters narrowing the v2 game browse. All are AND-combined; `None` means the
/// filter is not applied. `current_only` and `include_clones` default at the API
/// boundary, not here.
#[derive(Debug, Clone, Default)]
pub struct GameBrowseFilters {
	pub platform_id: Option<Uuid>,
	pub signature_group_id: Option<Uuid>,
	pub company_id: Option<Uuid>,
	pub current_only: bool,
	pub include_clones: bool,
}

fn games_browse_base(filters: &GameBrowseFilters) -> sea_orm::Select<Game> {
	let mut select = Game::find()
		.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
		.join(
			JoinType::InnerJoin,
			dat_file_import::Relation::DatFile.def(),
		)
		.join(JoinType::InnerJoin, dat_file::Relation::Platform.def());

	if let Some(platform_id) = filters.platform_id {
		select = select.filter(platform::Column::Id.eq(platform_id));
	}
	if let Some(signature_group_id) = filters.signature_group_id {
		select = select.filter(dat_file::Column::SignatureGroupId.eq(signature_group_id));
	}
	if let Some(company_id) = filters.company_id {
		select = select.filter(platform::Column::CompanyId.eq(company_id));
	}
	if filters.current_only {
		select = select.filter(game::Column::IsCurrent.eq(true));
	}
	if !filters.include_clones {
		select = select.filter(game::Column::CloneOf.is_null());
	}

	select
}

/// Fetch one keyset page of games ordered by `(name, id)`, narrowed by `filters`.
/// Seeks past `after` when supplied. The N+1 overflow row drives `has_more`.
pub async fn find_games_browse_page(
	filters: &GameBrowseFilters,
	after: Option<(String, Uuid)>,
	limit: Option<u64>,
	conn: &DbConn,
) -> Result<crate::db::pagination::KeysetPage<game::Model>, DbErr> {
	let mut cursor = games_browse_base(filters).cursor_by((game::Column::Name, game::Column::Id));
	if let Some((name, id)) = after {
		cursor.after((name, id));
	}
	crate::db::pagination::fetch_keyset_page(&mut cursor, limit, conn).await
}

/// One keyset page of the fuzzy game-name search ordered by similarity then id.
///
/// Unlike the v1 search the order key is the strict total order `(score DESC,
/// id ASC)` so a `(score, id)` cursor can seek deterministically; v1 keeps its
/// own ordering untouched. The score is `word_similarity(query, game.name)`,
/// recomputed for the seek predicate so paging never drifts. Returns rows as
/// `(score, id, name, platform_id, platform_name)`.
///
/// `limit` is capped at `GAME_NAME_SEARCH_MAX_LIMIT + 1` so the v2 caller can
/// request one extra row (the N+1 `has_more` probe) even on a full page.
#[allow(clippy::type_complexity)]
pub async fn search_games_by_name_page(
	query: &str,
	platform_id: Option<Uuid>,
	after: Option<(f64, Uuid)>,
	limit: u64,
	conn: &DbConn,
) -> Result<Vec<(f64, Uuid, String, Uuid, String)>, DbErr> {
	let capped_limit = limit.clamp(1, GAME_NAME_SEARCH_MAX_LIMIT + 1);
	let pattern = format!("%{query}%");

	let mut select = Game::find()
		.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
		.join(
			JoinType::InnerJoin,
			dat_file_import::Relation::DatFile.def(),
		)
		.join(JoinType::InnerJoin, dat_file::Relation::Platform.def())
		.filter(
			Expr::col((game::Entity, game::Column::Name))
				.ilike(pattern)
				.or(Expr::cust_with_values("game.name %> $1", [query])),
		);

	if let Some(platform_id) = platform_id {
		select = select.filter(platform::Column::Id.eq(platform_id));
	}

	// word_similarity returns real (FLOAT4); every use below is cast to float8 so the
	// score decodes into f64 and the (score, id) keyset cursor seeks in the same
	// double precision it was minted with.
	if let Some((score, id)) = after {
		let values: Vec<sea_orm::Value> = vec![query.into(), score.into(), id.into()];
		select = select.filter(Expr::cust_with_values(
			"(word_similarity($1, game.name)::float8 < $2 OR (word_similarity($1, game.name)::float8 = $2 AND game.id > $3))",
			values,
		));
	}

	select
		.select_only()
		.column_as(
			Expr::cust_with_values("word_similarity($1, game.name)::float8", [query]),
			"score",
		)
		.column(game::Column::Id)
		.column(game::Column::Name)
		.column_as(platform::Column::Id, "platform_id")
		.column_as(platform::Column::Name, "platform_name")
		.order_by_desc(Expr::cust_with_values(
			"word_similarity($1, game.name)::float8",
			[query],
		))
		.order_by_asc(game::Column::Id)
		.limit(capped_limit)
		.into_tuple::<(f64, Uuid, String, Uuid, String)>()
		.all(conn)
		.await
}

/// Matches on an exact file name first, then falls back to an exact game name.
pub async fn find_game_by_name_or_game_file_name(
	name: &str,
	conn: &DbConn,
) -> Result<Option<game::Model>, DbErr> {
	let game_file = GameFile::find()
		.filter(game_file::Column::FileName.eq(name))
		.find_also_related(Game)
		.one(conn)
		.await?;

	if let Some((_, Some(game))) = game_file {
		return Ok(Some(game));
	}

	let game = Game::find()
		.filter(game::Column::Name.eq(name))
		.one(conn)
		.await?;

	Ok(game)
}

async fn find_signature_metadata_mapping_if_exists_by_filter(
	input: SimpleExpr,
	conn: &DbConn,
) -> Result<Option<(game::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	// A hash can match several rows, either within one provider (a rename leaves
	// the retired entry beside the current one) or across providers that share a
	// content hash. Rank curated providers first via signature_group.display_priority,
	// then the current row, then a reconciled one (non-null last_seen) so a
	// never-reconciled orphan cannot win, then a stable id tiebreaker.
	let game_file = GameFile::find()
		.filter(input)
		.join(JoinType::InnerJoin, game_file::Relation::Game.def())
		.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
		.join(
			JoinType::InnerJoin,
			dat_file_import::Relation::DatFile.def(),
		)
		.join(
			JoinType::InnerJoin,
			dat_file::Relation::SignatureGroup.def(),
		)
		.order_by_asc(signature_group::Column::DisplayPriority)
		.order_by_desc(game_file::Column::IsCurrent)
		.order_by_desc(game_file::Column::LastSeenDatFileImportId.is_not_null())
		.order_by_asc(game_file::Column::Id)
		.one(conn)
		.await?;

	let Some(game_file) = game_file else {
		return Ok(None);
	};
	let Some(game) = Game::find_by_id(game_file.game_id).one(conn).await? else {
		return Ok(None);
	};

	let signature_metadata_mappings = signature_metadata_mapping::Entity::find()
		.filter(signature_metadata_mapping::Column::GameId.eq(game.id))
		.all(conn)
		.await?;

	Ok(Some((game, signature_metadata_mappings)))
}

/// Ranked sibling resolver behind the same hash filter as
/// [`find_signature_metadata_mapping_if_exists_by_filter`]. Where the
/// single-winner resolver collapses to one game, this returns every distinct
/// co-hashed game with its mappings, ordered by the same
/// `(display_priority, is_current, last_seen-not-null, id)` spine. Element zero
/// is provably the same game the single-winner resolver returns, so the V1
/// primary and this V2 union agree by construction.
async fn find_all_signature_metadata_mappings_by_filter(
	input: SimpleExpr,
	conn: &DbConn,
) -> Result<Vec<(game::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	let game_files = GameFile::find()
		.filter(input)
		.join(JoinType::InnerJoin, game_file::Relation::Game.def())
		.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
		.join(
			JoinType::InnerJoin,
			dat_file_import::Relation::DatFile.def(),
		)
		.join(
			JoinType::InnerJoin,
			dat_file::Relation::SignatureGroup.def(),
		)
		.order_by_asc(signature_group::Column::DisplayPriority)
		.order_by_desc(game_file::Column::IsCurrent)
		.order_by_desc(game_file::Column::LastSeenDatFileImportId.is_not_null())
		.order_by_asc(game_file::Column::Id)
		.all(conn)
		.await?;

	let mut seen = std::collections::HashSet::new();
	let mut ordered_ids = Vec::new();
	for game_file in &game_files {
		if seen.insert(game_file.game_id) {
			ordered_ids.push(game_file.game_id);
		}
	}

	let games = Game::find()
		.filter(game::Column::Id.is_in(ordered_ids.iter().copied()))
		.all(conn)
		.await?
		.into_iter()
		.map(|game| (game.id, game))
		.collect::<HashMap<_, _>>();
	let mut mappings_by_game =
		find_signature_metadata_mappings_by_game_ids(&ordered_ids, conn).await?;

	let mut result = Vec::with_capacity(ordered_ids.len());
	for id in ordered_ids {
		let Some(game) = games.get(&id) else {
			continue;
		};
		let signature_metadata_mappings = mappings_by_game.remove(&id).unwrap_or_default();
		result.push((game.clone(), signature_metadata_mappings));
	}

	Ok(result)
}

pub async fn find_all_games_and_id_mappings_by_md5(
	md5: &str,
	conn: &DbConn,
) -> Result<Vec<(game::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	find_all_signature_metadata_mappings_by_filter(game_file::Column::Md5.eq_ignore_case(md5), conn)
		.await
}

pub async fn find_all_games_and_id_mappings_by_sha1(
	sha1: &str,
	conn: &DbConn,
) -> Result<Vec<(game::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	find_all_signature_metadata_mappings_by_filter(
		game_file::Column::Sha1.eq_ignore_case(sha1),
		conn,
	)
	.await
}

pub async fn find_all_games_and_id_mappings_by_sha256(
	sha256: &str,
	conn: &DbConn,
) -> Result<Vec<(game::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	find_all_signature_metadata_mappings_by_filter(
		game_file::Column::Sha256.eq_ignore_case(sha256),
		conn,
	)
	.await
}

/// Ranked co-hashed lookup for the CRC rung, pinned to an exact size for the
/// same collision reason as [`find_game_and_id_mapping_by_crc`].
pub async fn find_all_games_and_id_mappings_by_crc(
	crc: &str,
	size: i64,
	conn: &DbConn,
) -> Result<Vec<(game::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	find_all_signature_metadata_mappings_by_filter(
		game_file::Column::Crc
			.eq_ignore_case(crc)
			.and(game_file::Column::FileSizeInBytes.eq(size)),
		conn,
	)
	.await
}

/// Distinct `game_id`s of every `game_file` whose hash for `match_type` equals
/// `value`, case-insensitively. Index-backed by `idx_game_file_lower_{col}`.
/// Used by the hash-aware cache bust to resolve the full co-hashed set for a
/// pure-addition import. `FileNameAndSize` and `NoMatch` are not hash inputs.
pub async fn find_game_ids_sharing_hash(
	match_type: crate::model::GameMatchType,
	value: &str,
	conn: &DbConn,
) -> Result<Vec<Uuid>, DbErr> {
	use crate::model::GameMatchType;
	let filter: SimpleExpr = match match_type {
		GameMatchType::SHA256 => game_file::Column::Sha256.eq_ignore_case(value),
		GameMatchType::SHA1 => game_file::Column::Sha1.eq_ignore_case(value),
		GameMatchType::MD5 => game_file::Column::Md5.eq_ignore_case(value),
		GameMatchType::CRC => game_file::Column::Crc.eq_ignore_case(value),
		GameMatchType::FileNameAndSize | GameMatchType::NoMatch => return Ok(Vec::new()),
	};

	GameFile::find()
		.select_only()
		.column(game_file::Column::GameId)
		.distinct()
		.filter(filter)
		.into_tuple::<Uuid>()
		.all(conn)
		.await
}

/// Map each game id in `game_ids` to its owning signature group's
/// `display_priority` via the `game -> dat_file_import -> dat_file ->
/// signature_group` chain. Drives the content-anchor survivorship tiebreak so
/// the read-path primary and the reconcile survivor agree on the same column.
pub async fn find_display_priorities_for_games(
	game_ids: &[Uuid],
	conn: &DbConn,
) -> Result<HashMap<Uuid, i16>, DbErr> {
	if game_ids.is_empty() {
		return Ok(HashMap::new());
	}

	let rows: Vec<(Uuid, i16)> = Game::find()
		.select_only()
		.column(game::Column::Id)
		.column(signature_group::Column::DisplayPriority)
		.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
		.join(
			JoinType::InnerJoin,
			dat_file_import::Relation::DatFile.def(),
		)
		.join(
			JoinType::InnerJoin,
			dat_file::Relation::SignatureGroup.def(),
		)
		.filter(game::Column::Id.is_in(game_ids.iter().copied()))
		.into_tuple::<(Uuid, i16)>()
		.all(conn)
		.await?;

	Ok(rows.into_iter().collect())
}

pub async fn find_all_children_of_game(
	game: &game::Model,
	conn: &DbConn,
) -> Result<Vec<game::Model>, DbErr> {
	Game::find()
		.filter(game::Column::CloneOf.eq(game.id))
		.all(conn)
		.await
}

pub async fn find_game_parent(
	game: &game::Model,
	conn: &DbConn,
) -> Result<Option<game::Model>, DbErr> {
	match game.clone_of {
		Some(clone_of_id) => {
			Game::find()
				.filter(game::Column::Id.eq(clone_of_id))
				.one(conn)
				.await
		}
		None => Ok(None),
	}
}

pub async fn find_game_signature_metadata_mapping(
	game: &game::Model,
	conn: &DbConn,
) -> Result<Option<signature_metadata_mapping::Model>, DbErr> {
	signature_metadata_mapping::Entity::find()
		.filter(signature_metadata_mapping::Column::GameId.eq(game.id))
		.one(conn)
		.await
}

pub async fn find_all_signature_metadata_mappings_for_game(
	game_id: Uuid,
	conn: &DbConn,
) -> Result<Vec<signature_metadata_mapping::Model>, DbErr> {
	signature_metadata_mapping::Entity::find()
		.filter(signature_metadata_mapping::Column::GameId.eq(game_id))
		.all(conn)
		.await
}

pub async fn get_dat_file_id_of_game(game: &game::Model, conn: &DbConn) -> Result<Uuid, DbErr> {
	let dat_file_import = dat_file_import::Entity::find()
		.filter(dat_file_import::Column::Id.eq(game.dat_file_import_id))
		.one(conn)
		.await?;

	match dat_file_import {
		Some(dat_file_import) => Ok(dat_file_import.dat_file_id),
		None => Err(DbErr::RecordNotFound(
			"Dat file import not found".to_string(),
		)),
	}
}
/// Whether the deferred clone-resolution pass still has unlinked rows for this
/// dat file: any game that knows its signature-group-internal clone-of id but
/// has no resolved `clone_of` UUID yet. When true, a clone graph built from the
/// resolved `clone_of` column may be missing edges the background pass has not
/// linked, so a reader should treat the graph as possibly incomplete.
pub async fn dat_file_has_unresolved_clones(
	dat_file_id: Uuid,
	conn: &DbConn,
) -> Result<bool, DbErr> {
	let unresolved = Game::find()
		.filter(
			game::Column::SignatureGroupInternalCloneOfId
				.is_not_null()
				.and(game::Column::CloneOf.is_null()),
		)
		.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
		.filter(dat_file_import::Column::DatFileId.eq(dat_file_id))
		.one(conn)
		.await?;
	Ok(unresolved.is_some())
}

/// Return every game that knows its signature-group-internal clone-of id but has no resolved `clone_of` UUID yet.
pub async fn get_unpopulated_clone_of_games(
	dat_file_id: Uuid,
	conn: &DbConn,
) -> Result<Vec<game::Model>, DbErr> {
	Game::find()
		.filter(
			game::Column::SignatureGroupInternalCloneOfId
				.is_not_null()
				.and(game::Column::CloneOf.is_null()),
		)
		.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
		.filter(dat_file_import::Column::DatFileId.eq(dat_file_id))
		.order_by_asc(game::Column::Id)
		.all(conn)
		.await
}

pub fn get_unmatched_games_without_clone_of_with_limit<'a>(
	provider: MetadataProviderEnum,
	page_size: u64,
	cursor: Option<Uuid>,
	conn: DbConn,
) -> BoxFuture<'a, anyhow::Result<Option<Vec<game::Model>>>> {
	get_unmatched_games_with_limit(provider, true, true, page_size, cursor, conn)
}

pub fn get_unmatched_games_with_clone_of_with_limit<'a>(
	provider: MetadataProviderEnum,
	page_size: u64,
	cursor: Option<Uuid>,
	conn: DbConn,
) -> BoxFuture<'a, anyhow::Result<Option<Vec<game::Model>>>> {
	get_unmatched_games_with_limit(provider, false, true, page_size, cursor, conn)
}

/// Same as [`get_unmatched_games_without_clone_of_with_limit`] but does not require the
/// game's platform to have a successful mapping for `provider`. Used by providers that
/// do not run a platform-matching pipeline (for example SteamGridDB, which searches by
/// game name without needing platform context).
pub fn get_unmatched_games_without_clone_of_with_limit_no_platform_gate<'a>(
	provider: MetadataProviderEnum,
	page_size: u64,
	cursor: Option<Uuid>,
	conn: DbConn,
) -> BoxFuture<'a, anyhow::Result<Option<Vec<game::Model>>>> {
	get_unmatched_games_with_limit(provider, true, false, page_size, cursor, conn)
}

/// Same as [`get_unmatched_games_with_clone_of_with_limit`] but without the platform
/// mapping gate. See [`get_unmatched_games_without_clone_of_with_limit_no_platform_gate`].
pub fn get_unmatched_games_with_clone_of_with_limit_no_platform_gate<'a>(
	provider: MetadataProviderEnum,
	page_size: u64,
	cursor: Option<Uuid>,
	conn: DbConn,
) -> BoxFuture<'a, anyhow::Result<Option<Vec<game::Model>>>> {
	get_unmatched_games_with_limit(provider, false, false, page_size, cursor, conn)
}

/// Min interval between cross-pass attempts on the same failed mapping.
/// Cross-pass results only change when sibling `matched_name` values churn,
/// which is typically a slow-moving signal. Re-running every cycle was the
/// CPU hot spot we are paying for.
pub const CROSS_MATCH_RETRY_INTERVAL_DAYS: i64 = 7;

/// Return up to `page_size` games where this `provider` is currently `Failed`
/// AND at least one sibling provider has matched the same game with a non-
/// null `matched_name` AND we have not cross-matched this row in the last
/// [`CROSS_MATCH_RETRY_INTERVAL_DAYS`]. Uses `EXISTS` (not a self-join) on
/// the sibling side so a game with multiple matched siblings appears once.
/// The new partial index `idx_smm_cross_match_pending` covers the outer
/// filter; `idx_smm_sibling_matched_name` covers the EXISTS subquery.
/// Returns `Ok(None)` when there is nothing left to process.
pub fn get_failed_games_for_cross_pass_with_limit<'a>(
	provider: MetadataProviderEnum,
	page_size: u64,
	cursor: Option<Uuid>,
	conn: DbConn,
) -> BoxFuture<'a, anyhow::Result<Option<Vec<game::Model>>>> {
	Box::pin(async move {
		let cooldown = Utc::now() - Duration::days(CROSS_MATCH_RETRY_INTERVAL_DAYS);
		let cooldown_naive: NaiveDateTime = cooldown.naive_utc();

		let mut query = Game::find()
			.join(
				JoinType::InnerJoin,
				game::Relation::SignatureMetadataMapping.def(),
			)
			.filter(
				signature_metadata_mapping::Column::Provider
					.eq(provider)
					.and(signature_metadata_mapping::Column::MatchType.eq(MatchTypeEnum::Failed))
					.and(
						signature_metadata_mapping::Column::FailedMatchReason
							.eq(FailedMatchReasonEnum::NoDirectMatch),
					)
					.and(
						signature_metadata_mapping::Column::CrossMatchLastTriedAt
							.is_null()
							.or(signature_metadata_mapping::Column::CrossMatchLastTriedAt
								.lt(cooldown_naive)),
					)
					.and(Expr::exists(
						sea_orm::sea_query::Query::select()
							.expr(Expr::val(1))
							.from(signature_metadata_mapping::Entity)
							.and_where(
								Expr::col(signature_metadata_mapping::Column::GameId)
									.equals((game::Entity, game::Column::Id)),
							)
							.and_where(
								Expr::col(signature_metadata_mapping::Column::Provider)
									.ne(provider.as_enum()),
							)
							.and_where(
								Expr::col(signature_metadata_mapping::Column::MatchType).is_in([
									MatchTypeEnum::Automatic.as_enum(),
									MatchTypeEnum::Manual.as_enum(),
								]),
							)
							.and_where(
								Expr::col(signature_metadata_mapping::Column::MatchedName)
									.is_not_null(),
							)
							.to_owned(),
					)),
			);

		if let Some(after) = cursor {
			query = query.filter(game::Column::Id.gt(after));
		}

		let res = query
			.order_by_asc(game::Column::Id)
			.limit(page_size)
			.all(&conn)
			.await?;

		if res.is_empty() {
			Ok(None)
		} else {
			Ok(Some(res))
		}
	})
}

/// Return up to `page_size` games whose last automatic match for `provider` failed with
/// `NoDirectMatch` more than 60 days ago. Used to retry stale failures.
/// Returns `Ok(None)` when there is nothing left to process.
pub fn get_automatic_match_failed_games_with_limit<'a>(
	provider: MetadataProviderEnum,
	page_size: u64,
	cursor: Option<Uuid>,
	conn: DbConn,
) -> BoxFuture<'a, anyhow::Result<Option<Vec<game::Model>>>> {
	Box::pin(async move {
		let sixty_days_ago = Utc::now() - Duration::days(60);
		let sixty_days_ago_naive: NaiveDateTime = sixty_days_ago.naive_utc();

		let mut query = Game::find()
			.join(
				JoinType::LeftJoin,
				game::Relation::SignatureMetadataMapping.def(),
			)
			.filter(
				signature_metadata_mapping::Column::MatchType
					.eq(MatchTypeEnum::Failed)
					.and(
						signature_metadata_mapping::Column::FailedMatchReason
							.eq(FailedMatchReasonEnum::NoDirectMatch),
					)
					.and(signature_metadata_mapping::Column::UpdatedAt.lt(sixty_days_ago_naive))
					.and(signature_metadata_mapping::Column::Provider.eq(provider)),
			);

		if let Some(after) = cursor {
			query = query.filter(game::Column::Id.gt(after));
		}

		let res = query
			.order_by_asc(game::Column::Id)
			.limit(page_size)
			.all(&conn)
			.await?;

		if res.is_empty() {
			Ok(None)
		} else {
			Ok(Some(res))
		}
	})
}

/// True if the provider has any game still needing a primary match: either an
/// unmatched game (no SMM row for this provider) or a stale
/// `Failed/NoDirectMatch` row past the 60-day retry window.
pub async fn has_outstanding_match_work_for_provider(
	provider: MetadataProviderEnum,
	conn: &DbConn,
) -> anyhow::Result<bool> {
	let unmatched_fut = async {
		let one = Game::find()
			.select_only()
			.column(game::Column::Id)
			.filter(
				Expr::exists(
					::sea_orm::sea_query::Query::select()
						.expr(Expr::val(1))
						.from(signature_metadata_mapping::Entity)
						.and_where(
							Expr::col(signature_metadata_mapping::Column::GameId)
								.equals((game::Entity, game::Column::Id)),
						)
						.and_where(
							Expr::col(signature_metadata_mapping::Column::Provider)
								.eq(provider.as_enum()),
						)
						.and_where(
							Expr::col(signature_metadata_mapping::Column::MatchType)
								.ne(MatchTypeEnum::None.as_enum()),
						)
						.to_owned(),
				)
				.not(),
			)
			.limit(1)
			.into_tuple::<Uuid>()
			.one(conn)
			.await?;
		anyhow::Ok(one.is_some())
	};

	let stale_failed_fut = async {
		let sixty_days_ago = Utc::now() - Duration::days(60);
		let sixty_days_ago_naive: NaiveDateTime = sixty_days_ago.naive_utc();
		let one = Game::find()
			.select_only()
			.column(game::Column::Id)
			.join(
				JoinType::InnerJoin,
				game::Relation::SignatureMetadataMapping.def(),
			)
			.filter(
				signature_metadata_mapping::Column::MatchType
					.eq(MatchTypeEnum::Failed)
					.and(
						signature_metadata_mapping::Column::FailedMatchReason
							.eq(FailedMatchReasonEnum::NoDirectMatch),
					)
					.and(signature_metadata_mapping::Column::UpdatedAt.lt(sixty_days_ago_naive))
					.and(signature_metadata_mapping::Column::Provider.eq(provider)),
			)
			.limit(1)
			.into_tuple::<Uuid>()
			.one(conn)
			.await?;
		anyhow::Ok(one.is_some())
	};

	let (unmatched, stale) = tokio::try_join!(unmatched_fut, stale_failed_fut)?;
	Ok(unmatched || stale)
}

/// True if the provider has any `Failed/NoDirectMatch` row whose cross-match
/// cooldown has elapsed and which has at least one sibling provider mapping
/// carrying a non-null `matched_name`.
pub async fn has_outstanding_cross_match_work_for_provider(
	provider: MetadataProviderEnum,
	conn: &DbConn,
) -> anyhow::Result<bool> {
	let cooldown = Utc::now() - Duration::days(CROSS_MATCH_RETRY_INTERVAL_DAYS);
	let cooldown_naive: NaiveDateTime = cooldown.naive_utc();

	let one = Game::find()
		.select_only()
		.column(game::Column::Id)
		.join(
			JoinType::InnerJoin,
			game::Relation::SignatureMetadataMapping.def(),
		)
		.filter(
			signature_metadata_mapping::Column::Provider
				.eq(provider)
				.and(signature_metadata_mapping::Column::MatchType.eq(MatchTypeEnum::Failed))
				.and(
					signature_metadata_mapping::Column::FailedMatchReason
						.eq(FailedMatchReasonEnum::NoDirectMatch),
				)
				.and(
					signature_metadata_mapping::Column::CrossMatchLastTriedAt
						.is_null()
						.or(signature_metadata_mapping::Column::CrossMatchLastTriedAt
							.lt(cooldown_naive)),
				)
				.and(Expr::exists(
					sea_orm::sea_query::Query::select()
						.expr(Expr::val(1))
						.from(signature_metadata_mapping::Entity)
						.and_where(
							Expr::col(signature_metadata_mapping::Column::GameId)
								.equals((game::Entity, game::Column::Id)),
						)
						.and_where(
							Expr::col(signature_metadata_mapping::Column::Provider)
								.ne(provider.as_enum()),
						)
						.and_where(
							Expr::col(signature_metadata_mapping::Column::MatchType).is_in([
								MatchTypeEnum::Automatic.as_enum(),
								MatchTypeEnum::Manual.as_enum(),
							]),
						)
						.and_where(
							Expr::col(signature_metadata_mapping::Column::MatchedName)
								.is_not_null(),
						)
						.to_owned(),
				)),
		)
		.limit(1)
		.into_tuple::<Uuid>()
		.one(conn)
		.await?;

	Ok(one.is_some())
}

fn get_unmatched_games_with_limit<'a>(
	provider: MetadataProviderEnum,
	clone_of_null: bool,
	require_platform_mapping: bool,
	page_size: u64,
	cursor: Option<Uuid>,
	conn: DbConn,
) -> BoxFuture<'a, anyhow::Result<Option<Vec<game::Model>>>> {
	Box::pin(async move {
		let smm1 = Alias::new("smm1");

		let mut query = Game::find()
			.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
			.join(
				JoinType::InnerJoin,
				dat_file_import::Relation::DatFile.def(),
			)
			.join(JoinType::InnerJoin, dat_file::Relation::Platform.def());

		if require_platform_mapping {
			query = query
				.join_as(
					JoinType::InnerJoin,
					platform::Relation::SignatureMetadataMapping.def(),
					smm1.clone(),
				)
				.filter(
					Expr::col((smm1.clone(), signature_metadata_mapping::Column::MatchType)).is_in(
						vec![
							MatchTypeEnum::Automatic.as_enum(),
							MatchTypeEnum::Manual.as_enum(),
						],
					),
				)
				.filter(
					Expr::col((smm1.clone(), signature_metadata_mapping::Column::Provider))
						.eq(provider.as_enum()),
				);
		}

		query = query
			.filter(if clone_of_null {
				game::Column::CloneOf.is_null()
			} else {
				game::Column::CloneOf.is_not_null()
			})
			.filter(
				Expr::exists(
					::sea_orm::sea_query::Query::select()
						.expr(Expr::val(1))
						.from(signature_metadata_mapping::Entity)
						.and_where(
							Expr::col(signature_metadata_mapping::Column::GameId)
								.equals((game::Entity, game::Column::Id)),
						)
						.and_where(
							Expr::col(signature_metadata_mapping::Column::Provider)
								.eq(provider.as_enum()),
						)
						.and_where(
							Expr::col(signature_metadata_mapping::Column::MatchType)
								.ne(MatchTypeEnum::None.as_enum()),
						)
						.to_owned(),
				)
				.not(),
			);

		if let Some(after) = cursor {
			query = query.filter(game::Column::Id.gt(after));
		}

		let res = query
			.order_by_asc(game::Column::Id)
			.limit(page_size)
			.all(&conn)
			.await?;

		if res.is_empty() {
			Ok(None)
		} else {
			Ok(Some(res))
		}
	})
}
