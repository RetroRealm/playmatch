use entity::{dat_file, game, game_file};
use sea_orm::prelude::Uuid;
use sea_orm::sea_query::Expr;
use sea_orm::{
	ColumnTrait, ConnectionTrait, DbBackend, DbConn, DbErr, EntityTrait, QueryFilter, Statement,
};

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
pub async fn reconcile_dat_file_lifecycle(
	dat_file_id: Uuid,
	import_id: Uuid,
	present_existing_file_ids: &[Uuid],
	game_ids: &[Uuid],
	conn: &DbConn,
) -> Result<(), DbErr> {
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

	// Retiring preserves the hash row and its last_seen pointer; only the cached
	// is_current flag flips, so a hash is never lost.
	conn.execute(Statement::from_sql_and_values(
		DbBackend::Postgres,
		r#"
		UPDATE game_file gf SET is_current = false
		FROM game g
		JOIN dat_file_import dfi ON dfi.id = g.dat_file_import_id
		WHERE g.id = gf.game_id
		  AND dfi.dat_file_id = $1
		  AND gf.is_current = true
		  AND gf.last_seen_dat_file_import_id IS DISTINCT FROM $2
		"#,
		[dat_file_id.into(), import_id.into()],
	))
	.await?;

	// Set-based so it also retires renamed and removed games, which the
	// name-based per-game loop never revisits.
	conn.execute(Statement::from_sql_and_values(
		DbBackend::Postgres,
		r#"
		UPDATE game g SET is_current = false
		FROM dat_file_import dfi
		WHERE dfi.id = g.dat_file_import_id
		  AND dfi.dat_file_id = $1
		  AND g.is_current = true
		  AND g.last_seen_dat_file_import_id IS DISTINCT FROM $2
		"#,
		[dat_file_id.into(), import_id.into()],
	))
	.await?;

	Ok(())
}
