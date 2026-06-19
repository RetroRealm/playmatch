use entity::{dat_file, dat_file_import, game, game_file};
use sea_orm::prelude::Uuid;
use sea_orm::sea_query::Expr;
use sea_orm::{
	ColumnTrait, ConnectionTrait, DbBackend, DbConn, DbErr, EntityTrait, JoinType, QueryFilter,
	QuerySelect, QueryTrait, RelationTrait, Statement,
};
use std::collections::HashSet;

/// Postgres caps bound parameters per statement; chunk id lists well under it.
const ID_CHUNK: usize = 8000;

/// Reconcile lifecycle state for a single dat file after one import has been
/// processed. Every write is set-based and idempotent, so it is safe under the
/// transaction-free, per-game parallel import path and converges on a retry.
///
/// `present_existing_file_ids` are game_file rows that already existed and were
/// seen again in this import. Newly inserted files already carry
/// `last_seen_dat_file_import_id = import_id` from insert time, so they are not
/// passed here. `game_ids` is every game touched by this import (existing and
/// new).
///
/// Returns the game ids whose currency changed, so the caller can bust their
/// identify cache entries.
pub async fn reconcile_dat_file_lifecycle(
	dat_file_id: Uuid,
	import_id: Uuid,
	present_existing_file_ids: &[Uuid],
	game_ids: &[Uuid],
	conn: &DbConn,
) -> Result<Vec<Uuid>, DbErr> {
	// The just-created import is always the newest, so "latest" is this import.
	dat_file::Entity::update_many()
		.col_expr(
			dat_file::Column::LatestDatFileImportId,
			Expr::value(import_id),
		)
		.filter(dat_file::Column::Id.eq(dat_file_id))
		.exec(conn)
		.await?;

	for chunk in present_existing_file_ids.chunks(ID_CHUNK) {
		game_file::Entity::update_many()
			.col_expr(
				game_file::Column::LastSeenDatFileImportId,
				Expr::value(import_id),
			)
			.col_expr(game_file::Column::IsCurrent, Expr::value(true))
			.filter(game_file::Column::Id.is_in(chunk.iter().copied()))
			.exec(conn)
			.await?;
	}

	for chunk in game_ids.chunks(ID_CHUNK) {
		game::Entity::update_many()
			.col_expr(
				game::Column::LastSeenDatFileImportId,
				Expr::value(import_id),
			)
			.col_expr(game::Column::IsCurrent, Expr::value(true))
			.filter(game::Column::Id.is_in(chunk.iter().copied()))
			.exec(conn)
			.await?;
	}

	// Newly inserted and bumped files all carry last_seen = import now, so this
	// set-based insert captures every present file without per-row id lists.
	conn.execute(Statement::from_sql_and_values(
		DbBackend::Postgres,
		r#"
		INSERT INTO game_file_presence (game_file_id, dat_file_import_id)
		SELECT gf.id, $2
		FROM game_file gf
		JOIN game g ON g.id = gf.game_id
		JOIN dat_file_import dfi ON dfi.id = g.dat_file_import_id
		WHERE dfi.dat_file_id = $1
		  AND gf.last_seen_dat_file_import_id = $2
		ON CONFLICT (game_file_id, dat_file_import_id) DO NOTHING
		"#,
		[dat_file_id.into(), import_id.into()],
	))
	.await?;

	// Retiring only flips is_current, so the hash row is never lost. The
	// last_seen filter is the null-safe form of IS DISTINCT FROM import_id.
	let mut retired_game_ids: HashSet<Uuid> = HashSet::new();

	let games_in_dat_file = game::Entity::find()
		.select_only()
		.column(game::Column::Id)
		.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
		.filter(dat_file_import::Column::DatFileId.eq(dat_file_id))
		.into_query();

	let retired_files = game_file::Entity::update_many()
		.col_expr(game_file::Column::IsCurrent, Expr::value(false))
		.filter(game_file::Column::GameId.in_subquery(games_in_dat_file))
		.filter(game_file::Column::IsCurrent.eq(true))
		.filter(
			game_file::Column::LastSeenDatFileImportId
				.is_null()
				.or(game_file::Column::LastSeenDatFileImportId.ne(import_id)),
		)
		.exec_with_returning(conn)
		.await?;
	for retired in retired_files {
		retired_game_ids.insert(retired.game_id);
	}

	// Set-based so it also retires renamed and removed games, which the
	// name-based per-game loop never revisits.
	let imports_of_dat_file = dat_file_import::Entity::find()
		.select_only()
		.column(dat_file_import::Column::Id)
		.filter(dat_file_import::Column::DatFileId.eq(dat_file_id))
		.into_query();

	let retired_games = game::Entity::update_many()
		.col_expr(game::Column::IsCurrent, Expr::value(false))
		.filter(game::Column::DatFileImportId.in_subquery(imports_of_dat_file))
		.filter(game::Column::IsCurrent.eq(true))
		.filter(
			game::Column::LastSeenDatFileImportId
				.is_null()
				.or(game::Column::LastSeenDatFileImportId.ne(import_id)),
		)
		.exec_with_returning(conn)
		.await?;
	for retired in retired_games {
		retired_game_ids.insert(retired.id);
	}

	Ok(retired_game_ids.into_iter().collect())
}
