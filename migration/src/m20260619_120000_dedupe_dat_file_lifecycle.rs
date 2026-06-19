use sea_orm::prelude::Uuid;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// A build-stamp duplicate dat_file and the canonical row it folds into.
struct DedupePair {
	duplicate_id: Uuid,
	canonical_id: Uuid,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	/// Merge the duplicate dat_file rows the historic build-stamp leak created
	/// back into their canonical sibling and recompute lifecycle. Idempotent.
	/// Only NULL-latest duplicates that have a canonical sibling are touched,
	/// which deliberately leaves the underscore Redump orphans and the live
	/// doubled-tag PSP row for a follow-up.
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let conn = manager.get_connection();

		let pairs = find_buildstamp_duplicates(conn).await?;
		if pairs.is_empty() {
			return Ok(());
		}

		for pair in &pairs {
			repoint_imports_to_canonical(conn, pair).await?;
		}
		for pair in &pairs {
			drop_emptied_duplicate(conn, pair).await?;
		}

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
		// One-way data backfill: the merged rows cannot be un-merged.
		Ok(())
	}
}

/// Pair every build-stamp duplicate with its canonical sibling. A duplicate is a
/// row whose name still carries a stripable stamp and has no latest pointer; the
/// canonical is the same name once the stamp is removed, scoped to the same
/// signature group, company and platform. The regex strips parenthetical groups
/// that are only digits and timestamp separators, the same rule the parser uses,
/// so variant tags such as (Decrypted) stay intact and distinct dats never merge.
async fn find_buildstamp_duplicates(conn: &impl ConnectionTrait) -> Result<Vec<DedupePair>, DbErr> {
	let rows = conn
		.query_all(Statement::from_string(
			DbBackend::Postgres,
			r#"
			WITH stripped AS (
				SELECT
					id,
					signature_group_id,
					company_id,
					platform_id,
					name,
					latest_dat_file_import_id,
					regexp_replace(name, ' \([0-9_ :-]*[0-9][0-9_ :-]*\)', '', 'g') AS stripped_name
				FROM dat_file
			),
			canonical AS (
				SELECT * FROM stripped
				WHERE name = stripped_name
				  AND latest_dat_file_import_id IS NOT NULL
			)
			SELECT DISTINCT ON (s.id) s.id AS duplicate_id, c.id AS canonical_id
			FROM stripped s
			JOIN canonical c
			  ON c.stripped_name = s.stripped_name
			 AND c.signature_group_id = s.signature_group_id
			 AND c.platform_id = s.platform_id
			 AND c.company_id IS NOT DISTINCT FROM s.company_id
			WHERE s.name <> s.stripped_name
			  AND s.latest_dat_file_import_id IS NULL
			  AND s.id <> c.id
			ORDER BY s.id, c.id
			"#
			.to_owned(),
		))
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
/// reconcile_dat_file_lifecycle; the merged orphans (older last_seen) flip false.
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
