use crate::db::abstraction::ColumnEqIgnoreCaseTrait;
use crate::db::pagination::{KeysetPage, fetch_keyset_page};
use crate::ingestion::parser::model::RomElement;
use entity::game::Entity as Game;
use entity::game_file;
use entity::game_file::ActiveModel;
use entity::game_file::Entity as GameFile;
use entity::{content_anchor, game};
use hex::encode as hex_encode;
use sea_orm::ActiveValue::Set;
use sea_orm::prelude::Uuid;
use sea_orm::sea_query::{Expr, OnConflict, SimpleExpr};
use sea_orm::{
	ActiveModelTrait, ColumnTrait, DbConn, DbErr, EntityTrait, IntoActiveModel, QueryFilter,
	QueryOrder, QuerySelect,
};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Resolve the single game file a hash matches, preferring the current row, then
/// a reconciled one (non-null `last_seen`), then a stable id tiebreaker. Mirrors
/// the ordering the identify cascade uses so a hash maps to the same row here.
async fn find_game_file_by_filter(
	filter: SimpleExpr,
	conn: &DbConn,
) -> Result<Option<game_file::Model>, DbErr> {
	GameFile::find()
		.filter(filter)
		.order_by_desc(game_file::Column::IsCurrent)
		.order_by_desc(game_file::Column::LastSeenDatFileImportId.is_not_null())
		.order_by_asc(game_file::Column::Id)
		.one(conn)
		.await
}

pub async fn find_game_file_by_sha256(
	sha256: &str,
	conn: &DbConn,
) -> Result<Option<game_file::Model>, DbErr> {
	find_game_file_by_filter(game_file::Column::Sha256.eq_ignore_case(sha256), conn).await
}

pub async fn find_game_file_by_sha1(
	sha1: &str,
	conn: &DbConn,
) -> Result<Option<game_file::Model>, DbErr> {
	find_game_file_by_filter(game_file::Column::Sha1.eq_ignore_case(sha1), conn).await
}

pub async fn find_game_file_by_md5(
	md5: &str,
	conn: &DbConn,
) -> Result<Option<game_file::Model>, DbErr> {
	find_game_file_by_filter(game_file::Column::Md5.eq_ignore_case(md5), conn).await
}

pub async fn find_game_file_by_crc(
	crc: &str,
	conn: &DbConn,
) -> Result<Option<game_file::Model>, DbErr> {
	find_game_file_by_filter(game_file::Column::Crc.eq_ignore_case(crc), conn).await
}

pub async fn insert_game_file_bulk(
	game_files: Vec<RomElement>,
	game_id: Uuid,
	import_id: Uuid,
	conn: &DbConn,
) -> anyhow::Result<()> {
	let mut to_insert = Vec::new();

	for game_file in game_files {
		let game_file = get_active_model_from_rom_element(game_id, import_id, game_file)?;

		to_insert.push(game_file);
	}

	game_file::Entity::insert_many(to_insert).exec(conn).await?;

	Ok(())
}

pub async fn insert_game_file(
	game_file: RomElement,
	game_id: Uuid,
	import_id: Uuid,
	conn: &DbConn,
) -> anyhow::Result<ActiveModel> {
	let game_file = get_active_model_from_rom_element(game_id, import_id, game_file)?;

	game_file.save(conn).await.map_err(|e| e.into())
}

pub async fn get_game_file_by_id(
	id: Uuid,
	conn: &DbConn,
) -> Result<Option<game_file::Model>, DbErr> {
	GameFile::find_by_id(id).one(conn).await
}

pub async fn get_game_files_from_game_id(
	game_id: Uuid,
	conn: &DbConn,
) -> Result<Vec<game_file::Model>, DbErr> {
	game_file::Entity::find()
		.filter(game_file::Column::GameId.eq(game_id))
		.all(conn)
		.await
}

/// One keyset page of a game's files ordered by `(file_name, id)`. Seeks past
/// `after` when supplied; `current_only` restricts to files still present in the
/// current dat release. The N+1 overflow row drives `has_more`.
pub async fn find_game_files_page(
	game_id: Uuid,
	current_only: bool,
	after: Option<(String, Uuid)>,
	limit: Option<u64>,
	conn: &DbConn,
) -> Result<KeysetPage<game_file::Model>, DbErr> {
	let mut select = GameFile::find().filter(game_file::Column::GameId.eq(game_id));
	if current_only {
		select = select.filter(game_file::Column::IsCurrent.eq(true));
	}

	let mut cursor = select.cursor_by((game_file::Column::FileName, game_file::Column::Id));
	if let Some((file_name, id)) = after {
		cursor.after((file_name, id));
	}
	fetch_keyset_page(&mut cursor, limit, conn).await
}

/// Bulk variant of [`get_game_files_from_game_id`]. Returns every game file whose
/// `game_id` is in `game_ids`; caller groups by `game_id` if needed.
pub async fn get_game_files_from_game_ids(
	game_ids: &[Uuid],
	conn: &DbConn,
) -> Result<Vec<game_file::Model>, DbErr> {
	if game_ids.is_empty() {
		return Ok(Vec::new());
	}
	game_file::Entity::find()
		.filter(game_file::Column::GameId.is_in(game_ids.iter().copied()))
		.all(conn)
		.await
}

fn get_active_model_from_rom_element(
	game_id: Uuid,
	import_id: Uuid,
	game_file: RomElement,
) -> anyhow::Result<ActiveModel> {
	let file_size = match game_file.size {
		None => None,
		Some(inner) => {
			if inner.is_empty() {
				None
			} else {
				Some(inner.parse::<i64>()?)
			}
		}
	};

	let game_file = ActiveModel {
		file_name: Set(game_file.name),
		file_size_in_bytes: Set(file_size),
		crc: Set(game_file.crc),
		md5: Set(game_file.md5),
		sha1: Set(game_file.sha1),
		sha256: Set(game_file.sha256),
		status: Set(game_file.status.map(|s| s.to_string())),
		serial: Set(game_file.serial),
		game_id: Set(game_id),
		last_seen_dat_file_import_id: Set(Some(import_id)),
		..Default::default()
	};
	Ok(game_file)
}

/// A game's content key is a digest over the sorted, lowercased SHA1 of all its
/// current `game_file` rows. Single-file is the one-element case of the same
/// rule. Returns `None` if any current file lacks a SHA1, which leaves the game
/// anchorless. `anchor_file_count` records the current file count for audit.
pub async fn compute_content_key(
	game: &game::Model,
	conn: &DbConn,
) -> Result<Option<(String, i16)>, DbErr> {
	let files = current_game_files(game.id, conn).await?;
	if files.is_empty() {
		return Ok(None);
	}

	let mut sha1s = Vec::with_capacity(files.len());
	for file in &files {
		match &file.sha1 {
			Some(sha1) => sha1s.push(sha1.to_lowercase()),
			None => return Ok(None),
		}
	}
	sha1s.sort();

	let mut hasher = Sha256::new();
	hasher.update(sha1s.join("\n").as_bytes());
	let anchor_hash = hex_encode(hasher.finalize());
	let anchor_file_count = files.len().min(i16::MAX as usize) as i16;

	Ok(Some((anchor_hash, anchor_file_count)))
}

async fn current_game_files(game_id: Uuid, conn: &DbConn) -> Result<Vec<game_file::Model>, DbErr> {
	game_file::Entity::find()
		.filter(game_file::Column::GameId.eq(game_id))
		.filter(game_file::Column::IsCurrent.eq(true))
		.all(conn)
		.await
}

/// Conflict-safe find-or-create on the unique `LOWER(anchor_hash)` index. The
/// `INSERT ... ON CONFLICT DO NOTHING` followed by a re-select tolerates two
/// co-hashed games racing onto the same anchor under the single replica.
async fn find_or_create_content_anchor(
	anchor_hash: &str,
	anchor_file_count: i16,
	conn: &DbConn,
) -> Result<Uuid, DbErr> {
	let active = content_anchor::ActiveModel {
		id: Set(Uuid::new_v4()),
		anchor_hash: Set(anchor_hash.to_string()),
		anchor_file_count: Set(anchor_file_count),
		..Default::default()
	};

	content_anchor::Entity::insert(active)
		.on_conflict(
			OnConflict::new()
				.expr(Expr::cust("LOWER(anchor_hash)"))
				.do_nothing()
				.to_owned(),
		)
		.do_nothing()
		.exec(conn)
		.await?;

	content_anchor::Entity::find()
		.filter(content_anchor::Column::AnchorHash.eq(anchor_hash))
		.one(conn)
		.await?
		.map(|a| a.id)
		.ok_or_else(|| DbErr::RecordNotFound("content_anchor".to_string()))
}

async fn anchor_member_game_ids(
	anchor_id: Uuid,
	exclude: Uuid,
	conn: &DbConn,
) -> Result<Vec<Uuid>, DbErr> {
	Game::find()
		.select_only()
		.column(game::Column::Id)
		.filter(game::Column::ContentAnchorId.eq(anchor_id))
		.filter(game::Column::Id.ne(exclude))
		.order_by_asc(game::Column::Id)
		.into_tuple::<Uuid>()
		.all(conn)
		.await
}

/// Present-hash disagreement on a SHA1-matched file vetoes a join (treat as an
/// engineered collision); absent hashes never veto. Compares the candidate
/// against one existing anchor member.
async fn secondary_hash_veto(
	candidate: &[game_file::Model],
	member_id: Uuid,
	conn: &DbConn,
) -> Result<bool, DbErr> {
	let member_files = current_game_files(member_id, conn).await?;
	let mut by_sha1: HashMap<String, &game_file::Model> = HashMap::new();
	for file in &member_files {
		if let Some(sha1) = &file.sha1 {
			by_sha1.insert(sha1.to_lowercase(), file);
		}
	}

	for file in candidate {
		let Some(sha1) = &file.sha1 else { continue };
		let Some(member) = by_sha1.get(&sha1.to_lowercase()) else {
			continue;
		};
		if hashes_disagree(&file.sha256, &member.sha256) || hashes_disagree(&file.md5, &member.md5)
		{
			return Ok(true);
		}
	}

	Ok(false)
}

fn hashes_disagree(a: &Option<String>, b: &Option<String>) -> bool {
	match (a, b) {
		(Some(a), Some(b)) => !a.eq_ignore_ascii_case(b),
		_ => false,
	}
}

/// Compute the game's content key, find-or-create its anchor conflict-safe, and
/// set `content_anchor_id`. Applies the secondary-hash veto before joining an
/// existing anchor: a present and disagreeing SHA256 or MD5 on a SHA1-matched
/// file leaves the game anchorless. Returns the anchor id when the game is
/// anchored, `None` when it is left anchorless.
pub async fn assign_content_anchor_for_game(
	game: &game::Model,
	conn: &DbConn,
) -> Result<Option<Uuid>, DbErr> {
	let Some((anchor_hash, anchor_file_count)) = compute_content_key(game, conn).await? else {
		return Ok(None);
	};

	let anchor_id = find_or_create_content_anchor(&anchor_hash, anchor_file_count, conn).await?;

	let members = anchor_member_game_ids(anchor_id, game.id, conn).await?;
	if let Some(member_id) = members.first() {
		let candidate = current_game_files(game.id, conn).await?;
		if secondary_hash_veto(&candidate, *member_id, conn).await? {
			return Ok(None);
		}
	}

	if game.content_anchor_id != Some(anchor_id) {
		let mut active = game.clone().into_active_model();
		active.content_anchor_id = Set(Some(anchor_id));
		active.update(conn).await?;
	}

	Ok(Some(anchor_id))
}

/// Distinct game ids sharing `anchor_id`, ordered by id. Drives the reconcile
/// wave and the seed-from-sibling copy.
pub async fn find_games_by_content_anchor(
	anchor_id: Uuid,
	conn: &DbConn,
) -> Result<Vec<game::Model>, DbErr> {
	Game::find()
		.filter(game::Column::ContentAnchorId.eq(anchor_id))
		.order_by_asc(game::Column::Id)
		.all(conn)
		.await
}

/// Games that have no content anchor yet, ordered by id. Drives the one-time
/// idempotent backfill; a freshly anchored game drops out of this set on the
/// next pass.
pub async fn get_games_without_content_anchor(conn: &DbConn) -> Result<Vec<game::Model>, DbErr> {
	Game::find()
		.filter(game::Column::ContentAnchorId.is_null())
		.order_by_asc(game::Column::Id)
		.all(conn)
		.await
}

/// Distinct anchor ids that carry more than one game. The reconcile wave only
/// needs to visit multi-member classes; single-member anchors converge to a
/// no-op.
pub async fn find_multi_member_anchor_ids(conn: &DbConn) -> Result<Vec<Uuid>, DbErr> {
	let rows: Vec<(Uuid, i64)> = Game::find()
		.select_only()
		.column(game::Column::ContentAnchorId)
		.column_as(game::Column::Id.count(), "member_count")
		.filter(game::Column::ContentAnchorId.is_not_null())
		.group_by(game::Column::ContentAnchorId)
		.having(Expr::expr(game::Column::Id.count()).gt(1))
		.into_tuple::<(Uuid, i64)>()
		.all(conn)
		.await?;
	Ok(rows.into_iter().map(|(id, _)| id).collect())
}
