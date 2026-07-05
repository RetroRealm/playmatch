use sea_orm::prelude::Uuid;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// A private-era redump dat_file and the public sibling it folds into.
struct DedupePair {
	duplicate_id: Uuid,
	canonical_id: Uuid,
}

/// SQL expression mapping a dat_file name to its public daily form: underscores
/// to spaces first (so stamps like (2025_07_19) still hit the paren regex), then
/// parenthesized build stamps and trailing bare date stamps stripped, whitespace
/// collapsed. The stamp classes are digits and separators only, the same rule as
/// the parser's is_build_stamp, so tags like (Decrypted) stay intact.
fn norm(col: &str) -> String {
	format!(
		r#"btrim(regexp_replace(regexp_replace(regexp_replace(replace({col}, '_', ' '), ' \([0-9 :-]*[0-9][0-9 :-]*\)', '', 'g'), '( -)? [0-9]{{4}}[0-9 :-]*$', ''), ' {{2,}}', ' ', 'g'))"#
	)
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	/// Fold the private-era Redump dat_file rows into their clean-named public
	/// sibling and recompute lifecycle. Private dats were always imported under
	/// the shared 'Redump' signature group, so the residue is rows with noisy
	/// names: duplicates merge into their public sibling, counterpart-less
	/// orphans are renamed in place and never deleted. Idempotent, a no-op on
	/// fresh databases, one-way (down is a no-op).
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let conn = manager.get_connection();

		let pairs = find_private_duplicates(conn).await?;

		for pair in &pairs {
			repoint_imports_to_canonical(conn, pair).await?;
		}
		for pair in &pairs {
			drop_emptied_duplicate(conn, pair).await?;
		}

		rename_orphans_to_public_names(conn).await?;

		let mut canonicals: Vec<Uuid> = pairs.iter().map(|p| p.canonical_id).collect();
		canonicals.sort();
		canonicals.dedup();
		for canonical_id in canonicals {
			recompute_latest_pointer(conn, canonical_id).await?;
			backfill_missing_last_seen(conn, canonical_id).await?;
			recompute_currency(conn, canonical_id).await?;
		}

		Ok(())
	}

	async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
		// One-way data merge: the folded redump-private rows cannot be un-merged.
		Ok(())
	}
}

/// Pair every private-era Redump dat_file with its public sibling. Both sides
/// may be live, so the canonical is chosen by preference: the clean public name
/// (future public imports look it up by exact name), then the newest import,
/// then the smallest id. Rows are grouped by normalized name and scoped to the
/// same signature group, platform and company, so distinct dats never merge.
async fn find_private_duplicates(conn: &impl ConnectionTrait) -> Result<Vec<DedupePair>, DbErr> {
	let sql = format!(
		r#"
		WITH redump AS (
			SELECT d.id, d.name, d.signature_group_id, d.company_id, d.platform_id,
			       {norm_name} AS norm_name,
			       (SELECT max(i.imported_at) FROM dat_file_import i WHERE i.dat_file_id = d.id) AS newest_import
			FROM dat_file d
			JOIN signature_group sg ON sg.id = d.signature_group_id
			WHERE sg.name = 'Redump'
		),
		canonical AS (
			SELECT DISTINCT ON (norm_name, signature_group_id, platform_id, company_id) *
			FROM redump
			ORDER BY norm_name, signature_group_id, platform_id, company_id,
			         (name = norm_name) DESC,
			         newest_import DESC NULLS LAST,
			         id
		)
		SELECT r.id AS duplicate_id, c.id AS canonical_id
		FROM redump r
		JOIN canonical c
		  ON c.norm_name = r.norm_name
		 AND c.signature_group_id = r.signature_group_id
		 AND c.platform_id = r.platform_id
		 AND c.company_id IS NOT DISTINCT FROM r.company_id
		WHERE r.id <> c.id
		ORDER BY r.id
		"#,
		norm_name = norm("d.name"),
	);

	let rows = conn
		.query_all(Statement::from_string(DbBackend::Postgres, sql))
		.await?;

	rows.into_iter()
		.map(|row| {
			Ok(DedupePair {
				duplicate_id: row.try_get("", "duplicate_id")?,
				canonical_id: row.try_get("", "canonical_id")?,
			})
		})
		.collect()
}

/// Move the duplicate's imports onto the canonical. Games and game files follow
/// implicitly through their unchanged dat_file_import_id.
async fn repoint_imports_to_canonical(
	conn: &impl ConnectionTrait,
	pair: &DedupePair,
) -> Result<(), DbErr> {
	conn.execute(Statement::from_sql_and_values(
		DbBackend::Postgres,
		"UPDATE dat_file_import SET dat_file_id = $1 WHERE dat_file_id = $2",
		[pair.canonical_id.into(), pair.duplicate_id.into()],
	))
	.await?;
	Ok(())
}

/// Drop the now-empty duplicate. The NOT EXISTS guard keeps this a no-op if a
/// stray import remains, so the cascade can never delete real games.
async fn drop_emptied_duplicate(
	conn: &impl ConnectionTrait,
	pair: &DedupePair,
) -> Result<(), DbErr> {
	conn.execute(Statement::from_sql_and_values(
		DbBackend::Postgres,
		r#"
		DELETE FROM dat_file
		WHERE id = $1
		  AND NOT EXISTS (SELECT 1 FROM dat_file_import WHERE dat_file_id = $1)
		"#,
		[pair.duplicate_id.into()],
	))
	.await?;
	Ok(())
}

/// Rename a private row without a public counterpart to its normalized public
/// name so create_or_update_dat_file's exact-name lookup continues it on the
/// next public import; the row is preserved, not deleted. The NOT EXISTS guard
/// stops an interrupted earlier run from producing two same-named rows, and the
/// '' guard skips a pathological all-noise name.
async fn rename_orphans_to_public_names(conn: &impl ConnectionTrait) -> Result<(), DbErr> {
	let sql = format!(
		r#"
		UPDATE dat_file d
		SET name = {norm_d}
		FROM signature_group sg
		WHERE sg.id = d.signature_group_id
		  AND sg.name = 'Redump'
		  AND d.name <> {norm_d}
		  AND {norm_d} <> ''
		  AND NOT EXISTS (
		      SELECT 1 FROM dat_file o
		      WHERE o.id <> d.id
		        AND o.signature_group_id = d.signature_group_id
		        AND o.platform_id = d.platform_id
		        AND o.company_id IS NOT DISTINCT FROM d.company_id
		        AND {norm_o} = {norm_d}
		  )
		"#,
		norm_d = norm("d.name"),
		norm_o = norm("o.name"),
	);

	conn.execute(Statement::from_string(DbBackend::Postgres, sql))
		.await?;
	Ok(())
}

/// Point the canonical at its newest import across the merged set.
async fn recompute_latest_pointer(
	conn: &impl ConnectionTrait,
	canonical_id: Uuid,
) -> Result<(), DbErr> {
	conn.execute(Statement::from_sql_and_values(
		DbBackend::Postgres,
		r#"
		UPDATE dat_file
		SET latest_dat_file_import_id = (
			SELECT id FROM dat_file_import
			WHERE dat_file_id = $1
			ORDER BY imported_at DESC, id DESC
			LIMIT 1
		)
		WHERE id = $1
		"#,
		[canonical_id.into()],
	))
	.await?;
	Ok(())
}

/// Fill last_seen for rows predating the lifecycle columns (NULL) from the
/// import that owns them, so the currency recompute has a value to compare.
async fn backfill_missing_last_seen(
	conn: &impl ConnectionTrait,
	canonical_id: Uuid,
) -> Result<(), DbErr> {
	conn.execute(Statement::from_sql_and_values(
		DbBackend::Postgres,
		r#"
		UPDATE game_file gf
		SET last_seen_dat_file_import_id = g.dat_file_import_id
		FROM game g
		JOIN dat_file_import dfi ON dfi.id = g.dat_file_import_id
		WHERE gf.game_id = g.id
		  AND gf.last_seen_dat_file_import_id IS NULL
		  AND dfi.dat_file_id = $1
		"#,
		[canonical_id.into()],
	))
	.await?;

	conn.execute(Statement::from_sql_and_values(
		DbBackend::Postgres,
		r#"
		UPDATE game g
		SET last_seen_dat_file_import_id = g.dat_file_import_id
		FROM dat_file_import dfi
		WHERE dfi.id = g.dat_file_import_id
		  AND g.last_seen_dat_file_import_id IS NULL
		  AND dfi.dat_file_id = $1
		"#,
		[canonical_id.into()],
	))
	.await?;
	Ok(())
}

/// A row is current iff it was last seen in its dat file's latest import. Mirrors
/// reconcile_dat_file_lifecycle; the merged private rows (older last_seen) flip false.
async fn recompute_currency(conn: &impl ConnectionTrait, canonical_id: Uuid) -> Result<(), DbErr> {
	conn.execute(Statement::from_sql_and_values(
		DbBackend::Postgres,
		r#"
		UPDATE game_file gf
		SET is_current = (gf.last_seen_dat_file_import_id IS NOT DISTINCT FROM d.latest_dat_file_import_id)
		FROM game g
		JOIN dat_file_import dfi ON dfi.id = g.dat_file_import_id
		JOIN dat_file d ON d.id = dfi.dat_file_id
		WHERE gf.game_id = g.id
		  AND d.id = $1
		"#,
		[canonical_id.into()],
	))
	.await?;

	conn.execute(Statement::from_sql_and_values(
		DbBackend::Postgres,
		r#"
		UPDATE game g
		SET is_current = (g.last_seen_dat_file_import_id IS NOT DISTINCT FROM d.latest_dat_file_import_id)
		FROM dat_file_import dfi
		JOIN dat_file d ON d.id = dfi.dat_file_id
		WHERE dfi.id = g.dat_file_import_id
		  AND d.id = $1
		"#,
		[canonical_id.into()],
	))
	.await?;
	Ok(())
}
