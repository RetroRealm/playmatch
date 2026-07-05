//! Postgres-backed tests for dat_file dedup and lifecycle currency.
//! Require Docker.

use migration::{Migrator, MigratorTrait};
use sea_orm::prelude::Uuid;
use sea_orm::{ConnectionTrait, Database, DbConn, Statement};
use service::db::game::find_game_and_id_mapping_by_sha1;
use service::db::lifecycle::reconcile_dat_file_lifecycle;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use testcontainers_modules::testcontainers::{ContainerAsync, ImageExt};

/// The returned container guard must be kept alive for the test's duration.
async fn start_pg() -> (ContainerAsync<Postgres>, DbConn) {
	// Matches the production major version.
	let container = Postgres::default()
		.with_tag("18-alpine")
		.start()
		.await
		.unwrap();
	let port = container.get_host_port_ipv4(5432).await.unwrap();
	let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
	let db = Database::connect(&url).await.unwrap();
	Migrator::up(&db, None).await.unwrap();
	(container, db)
}

/// Number of trailing migrations to roll back so that `name` and everything
/// registered after it are undone, regardless of how many migrations follow.
fn rollback_steps_through(name: &str) -> u32 {
	let migrations = Migrator::migrations();
	let index = migrations
		.iter()
		.position(|m| m.name() == name)
		.unwrap_or_else(|| panic!("migration {name} must be registered"));
	(migrations.len() - index) as u32
}

const SHA1: &str = "432dbe312bc51e36bb8cb6fcb5e08f6968f124a4";

const SG: &str = "11111111-1111-1111-1111-111111111111";
const PLAT: &str = "22222222-2222-2222-2222-222222222222";
const CANON_DF: &str = "a1a1a1a1-a1a1-a1a1-a1a1-a1a1a1a1a1a1";
const ORPHAN_DF: &str = "b2b2b2b2-b2b2-b2b2-b2b2-b2b2b2b2b2b2";
const CANON_IMPORT: &str = "c3c3c3c3-c3c3-c3c3-c3c3-c3c3c3c3c3c3";
const ORPHAN_IMPORT: &str = "d4d4d4d4-d4d4-d4d4-d4d4-d4d4d4d4d4d4";
const CANON_GAME: &str = "e5e5e5e5-e5e5-e5e5-e5e5-e5e5e5e5e5e5";
const ORPHAN_GAME: &str = "f6f6f6f6-f6f6-f6f6-f6f6-f6f6f6f6f6f6";
// Orphan gets the smaller id so a plain `id ASC` would pick it; the test then
// proves the last_seen tiebreaker still selects the canonical row.
const CANON_GF: &str = "99999999-9999-9999-9999-999999999999";
const ORPHAN_GF: &str = "07070707-0707-0707-0707-070707070707";

#[tokio::test]
async fn dedupe_migration_merges_buildstamp_orphan_and_fixes_currency() {
	let (_pg, db) = start_pg().await;
	seed_orphan_scenario(&db).await;

	assert_eq!(
		count_ds_decrypted(&db).await,
		2,
		"seed should hold both rows"
	);

	let (before, _) = find_game_and_id_mapping_by_sha1(SHA1, &db)
		.await
		.unwrap()
		.expect("sha1 should resolve before migration");
	assert_eq!(
		before.id.to_string(),
		CANON_GAME,
		"tiebreaker must prefer the reconciled row even though the orphan has a smaller id"
	);

	// Re-apply the dedupe migration over the seeded data (its down is a no-op).
	// Compute how many trailing migrations to roll back so the count stays correct
	// as migrations are appended after the dedupe one.
	let steps = rollback_steps_through("m20260619_120000_dedupe_dat_file_lifecycle");
	Migrator::down(&db, Some(steps)).await.unwrap();
	Migrator::up(&db, None).await.unwrap();

	assert_eq!(
		count_ds_decrypted(&db).await,
		1,
		"the build-stamp duplicate must be merged into the canonical row"
	);

	let (game, _) = find_game_and_id_mapping_by_sha1(SHA1, &db)
		.await
		.unwrap()
		.expect("sha1 should still resolve to a game");

	assert!(
		game.is_current,
		"the sha1 must resolve to the current game, not the retired orphan"
	);

	let latest = dat_file_latest_for_game(&db, game.id).await;
	assert!(latest.is_some(), "the dat file must have a latest import");
	assert_eq!(
		game.last_seen_dat_file_import_id, latest,
		"current_in_latest_dat must be true: last_seen must equal the dat file's latest import"
	);
}

async fn seed_orphan_scenario(db: &DbConn) {
	let sql = format!(
		r#"
		INSERT INTO signature_group (id, name) VALUES ('{SG}', 'No-Intro');
		INSERT INTO platform (id, name) VALUES ('{PLAT}', 'Nintendo DS');

		INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id, latest_dat_file_import_id)
		VALUES ('{CANON_DF}', 'Nintendo - Nintendo DS (Decrypted)', '{PLAT}', '20260617-122122', '{SG}', '{CANON_IMPORT}');
		INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id, latest_dat_file_import_id)
		VALUES ('{ORPHAN_DF}', 'Nintendo - Nintendo DS (Decrypted) (20260605-234217)', '{PLAT}', '20260605-170618', '{SG}', NULL);

		INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at)
		VALUES ('{CANON_IMPORT}', '{CANON_DF}', 'Nintendo - Nintendo DS (Decrypted) (20260617-122122).dat', '20260617-122122', 'aaaaaaaa', '2026-06-18 12:00:00+00');
		INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at)
		VALUES ('{ORPHAN_IMPORT}', '{ORPHAN_DF}', 'Nintendo - Nintendo DS (Decrypted) (20260605-234217).dat', '20260605-170618', 'bbbbbbbb', '2026-06-06 12:00:00+00');

		INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id)
		VALUES ('{CANON_GAME}', '{CANON_IMPORT}', 'Pokemon - Diamant-Edition (Germany) (Rev 5)', true, '{CANON_IMPORT}');
		INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id)
		VALUES ('{ORPHAN_GAME}', '{ORPHAN_IMPORT}', 'Pokemon - Diamant-Edition (Germany) (Rev 5)', true, NULL);

		INSERT INTO game_file (id, game_id, file_name, sha1, is_current, last_seen_dat_file_import_id)
		VALUES ('{CANON_GF}', '{CANON_GAME}', 'Pokemon - Diamant-Edition (Germany) (Rev 5).nds', '{SHA1}', true, '{CANON_IMPORT}');
		INSERT INTO game_file (id, game_id, file_name, sha1, is_current, last_seen_dat_file_import_id)
		VALUES ('{ORPHAN_GF}', '{ORPHAN_GAME}', 'Pokemon - Diamant-Edition (Germany) (Rev 5).nds', '{SHA1}', true, NULL);
		"#
	);
	db.execute_unprepared(&sql).await.unwrap();
}

async fn count_ds_decrypted(db: &DbConn) -> i64 {
	let row = db
		.query_one(Statement::from_string(
			db.get_database_backend(),
			"SELECT count(*) AS cnt FROM dat_file WHERE name LIKE 'Nintendo - Nintendo DS (Decrypted)%'"
				.to_owned(),
		))
		.await
		.unwrap()
		.unwrap();
	row.try_get::<i64>("", "cnt").unwrap()
}

async fn dat_file_latest_for_game(db: &DbConn, game_id: Uuid) -> Option<Uuid> {
	let row = db
		.query_one(Statement::from_string(
			db.get_database_backend(),
			format!(
				r#"
				SELECT d.latest_dat_file_import_id AS latest
				FROM game g
				JOIN dat_file_import dfi ON dfi.id = g.dat_file_import_id
				JOIN dat_file d ON d.id = dfi.dat_file_id
				WHERE g.id = '{game_id}'
				"#
			),
		))
		.await
		.unwrap()
		.unwrap();
	row.try_get::<Option<Uuid>>("", "latest").unwrap()
}

const R_SG: &str = "33333333-3333-3333-3333-333333333333";
const R_PLAT: &str = "44444444-4444-4444-4444-444444444444";
const R_DF: &str = "55555555-5555-5555-5555-555555555555";
const R_IMPORT1: &str = "61616161-6161-6161-6161-616161616161";
const R_IMPORT2: &str = "62626262-6262-6262-6262-626262626262";
const R_G1_VANISHED: &str = "71717171-7171-7171-7171-717171717171";
const R_G2_PRESENT: &str = "72727272-7272-7272-7272-727272727272";
const R_F1_VANISHED: &str = "81818181-8181-8181-8181-818181818181";
const R_F2_PRESENT: &str = "82828282-8282-8282-8282-828282828282";

// A second import re-sees only the present game; the vanished one must retire
// and come back in the returned ids, the present one must stay current.
#[tokio::test]
async fn reconcile_retires_vanished_rows_and_returns_their_game_ids() {
	let (_pg, db) = start_pg().await;
	seed_reconcile_scenario(&db).await;

	let retired: std::collections::HashSet<String> = reconcile_dat_file_lifecycle(
		Uuid::parse_str(R_DF).unwrap(),
		Uuid::parse_str(R_IMPORT2).unwrap(),
		&[Uuid::parse_str(R_F2_PRESENT).unwrap()],
		&[Uuid::parse_str(R_G2_PRESENT).unwrap()],
		&db,
	)
	.await
	.unwrap()
	.into_iter()
	.map(|id| id.to_string())
	.collect();

	assert!(
		retired.contains(R_G1_VANISHED),
		"the vanished game must be reported retired for cache busting"
	);
	assert!(
		!retired.contains(R_G2_PRESENT),
		"the still-present game must not be reported retired"
	);

	assert!(!is_current_game(&db, R_G1_VANISHED).await);
	assert!(!is_current_file(&db, R_F1_VANISHED).await);
	assert!(is_current_game(&db, R_G2_PRESENT).await);
	assert!(is_current_file(&db, R_F2_PRESENT).await);

	assert_eq!(
		dat_file_latest_for_game(&db, Uuid::parse_str(R_G2_PRESENT).unwrap()).await,
		Some(Uuid::parse_str(R_IMPORT2).unwrap()),
		"reconcile must advance the dat file's latest pointer to the new import"
	);
}

async fn seed_reconcile_scenario(db: &DbConn) {
	let sql = format!(
		r#"
		INSERT INTO signature_group (id, name) VALUES ('{R_SG}', 'No-Intro');
		INSERT INTO platform (id, name) VALUES ('{R_PLAT}', 'Nintendo DS');

		INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id, latest_dat_file_import_id)
		VALUES ('{R_DF}', 'Nintendo - Nintendo DS (Decrypted)', '{R_PLAT}', '20260606-000000', '{R_SG}', '{R_IMPORT1}');

		INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at)
		VALUES ('{R_IMPORT1}', '{R_DF}', 'old.dat', '20260606-000000', 'oldoldol', '2026-06-06 12:00:00+00');
		INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at)
		VALUES ('{R_IMPORT2}', '{R_DF}', 'new.dat', '20260618-000000', 'newnewne', '2026-06-18 12:00:00+00');

		INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id)
		VALUES ('{R_G1_VANISHED}', '{R_IMPORT1}', 'Vanished Game', true, '{R_IMPORT1}');
		INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id)
		VALUES ('{R_G2_PRESENT}', '{R_IMPORT1}', 'Present Game', true, '{R_IMPORT1}');

		INSERT INTO game_file (id, game_id, file_name, sha1, is_current, last_seen_dat_file_import_id)
		VALUES ('{R_F1_VANISHED}', '{R_G1_VANISHED}', 'vanished.nds', 'aaaa', true, '{R_IMPORT1}');
		INSERT INTO game_file (id, game_id, file_name, sha1, is_current, last_seen_dat_file_import_id)
		VALUES ('{R_F2_PRESENT}', '{R_G2_PRESENT}', 'present.nds', 'bbbb', true, '{R_IMPORT1}');
		"#
	);
	db.execute_unprepared(&sql).await.unwrap();
}

async fn is_current_game(db: &DbConn, id: &str) -> bool {
	current_flag(db, "game", id).await
}

async fn is_current_file(db: &DbConn, id: &str) -> bool {
	current_flag(db, "game_file", id).await
}

async fn current_flag(db: &DbConn, table: &str, id: &str) -> bool {
	let row = db
		.query_one(Statement::from_string(
			db.get_database_backend(),
			format!("SELECT is_current FROM {table} WHERE id = '{id}'"),
		))
		.await
		.unwrap()
		.unwrap();
	row.try_get::<bool>("", "is_current").unwrap()
}

const M_PLAT: &str = "aa000000-0000-0000-0000-000000000001";
const M_PLAT2: &str = "aa000000-0000-0000-0000-000000000002";
const M_PLAT3: &str = "aa000000-0000-0000-0000-000000000003";
const PUB_DF: &str = "ab000000-0000-0000-0000-000000000001";
const PRIV_DF: &str = "ab000000-0000-0000-0000-000000000002";
const ORPH_DF: &str = "ab000000-0000-0000-0000-000000000003";
const MERGED_DF: &str = "ab000000-0000-0000-0000-000000000004";
const I_PUB: &str = "ac000000-0000-0000-0000-000000000001";
const I_PRIV: &str = "ac000000-0000-0000-0000-000000000002";
const I_ORPH: &str = "ac000000-0000-0000-0000-000000000003";
const I_MERGED: &str = "ac000000-0000-0000-0000-000000000004";
const PUB_GAME: &str = "ad000000-0000-0000-0000-000000000001";
const PRIV_GAME: &str = "ad000000-0000-0000-0000-000000000002";
const ORPH_GAME: &str = "ad000000-0000-0000-0000-000000000003";
const MERGED_GAME: &str = "ad000000-0000-0000-0000-000000000004";
const PUB_GF: &str = "ae000000-0000-0000-0000-000000000001";
const PRIV_GF: &str = "ae000000-0000-0000-0000-000000000002";
const ORPH_GF: &str = "ae000000-0000-0000-0000-000000000003";
const MERGED_GF: &str = "ae000000-0000-0000-0000-000000000004";

// A live private-era "Nintendo - Wii U (2025-07-19 10-00-00)" folds into its
// clean public sibling, the counterpart-less underscore/date-stamped Xbox 360
// dat is renamed in place, and an already-deduplicated lone "Sony -
// PlayStation 3" passes through untouched. Re-running must be a no-op.
#[tokio::test]
async fn merge_migration_folds_private_dat_into_public_and_renames_orphans() {
	let (_pg, db) = start_pg().await;
	let sg = redump_sg_id(&db).await;
	seed_private_merge_scenario(&db, &sg).await;

	run_merge_migration(&db).await;
	assert_merged_state(&db).await;

	// Re-run over already-merged data to prove idempotency.
	run_merge_migration(&db).await;
	assert_merged_state(&db).await;
}

async fn run_merge_migration(db: &DbConn) {
	let steps = rollback_steps_through("m20260705_120000_merge_redump_private_dat_files");
	Migrator::down(db, Some(steps)).await.unwrap();
	Migrator::up(db, None).await.unwrap();
}

async fn assert_merged_state(db: &DbConn) {
	assert_eq!(
		count_named(db, "Nintendo - Wii U").await,
		1,
		"the private Wii U duplicate must be merged into the public row"
	);
	assert_eq!(
		scalar_uuid(
			db,
			"SELECT id AS v FROM dat_file WHERE name = 'Nintendo - Wii U'"
		)
		.await,
		Some(Uuid::parse_str(PUB_DF).unwrap()),
		"the surviving Wii U row must be the public canonical"
	);
	assert!(
		dat_file_name(db, PRIV_DF).await.is_none(),
		"the emptied private duplicate must be deleted"
	);
	assert_eq!(
		scalar_uuid(
			db,
			&format!("SELECT dat_file_id AS v FROM dat_file_import WHERE id = '{I_PRIV}'"),
		)
		.await,
		Some(Uuid::parse_str(PUB_DF).unwrap()),
		"the private import must be repointed to the public row"
	);
	assert_eq!(
		scalar_uuid(
			db,
			&format!("SELECT latest_dat_file_import_id AS v FROM dat_file WHERE id = '{PUB_DF}'"),
		)
		.await,
		Some(Uuid::parse_str(I_PUB).unwrap()),
		"the canonical must point at the newest import"
	);

	assert!(!is_current_game(db, PRIV_GAME).await);
	assert!(!is_current_file(db, PRIV_GF).await);
	assert!(is_current_game(db, PUB_GAME).await);
	assert!(is_current_file(db, PUB_GF).await);

	assert_eq!(
		dat_file_name(db, ORPH_DF).await.as_deref(),
		Some("Microsoft - Xbox 360"),
		"the counterpart-less orphan must be renamed, not deleted"
	);
	assert!(is_current_game(db, ORPH_GAME).await);
	assert!(is_current_file(db, ORPH_GF).await);
	assert_eq!(
		scalar_uuid(
			db,
			&format!("SELECT dat_file_id AS v FROM dat_file_import WHERE id = '{I_ORPH}'"),
		)
		.await,
		Some(Uuid::parse_str(ORPH_DF).unwrap()),
		"the orphan's import must stay put"
	);

	assert_eq!(
		dat_file_name(db, MERGED_DF).await.as_deref(),
		Some("Sony - PlayStation 3"),
		"an already-deduplicated single dat must keep its name"
	);
	assert_eq!(
		scalar_uuid(
			db,
			&format!(
				"SELECT latest_dat_file_import_id AS v FROM dat_file WHERE id = '{MERGED_DF}'"
			),
		)
		.await,
		Some(Uuid::parse_str(I_MERGED).unwrap()),
		"an already-deduplicated single dat must keep its latest pointer"
	);
	assert!(is_current_game(db, MERGED_GAME).await);
	assert!(is_current_file(db, MERGED_GF).await);
}

async fn redump_sg_id(db: &DbConn) -> String {
	scalar_uuid(
		db,
		"SELECT id AS v FROM signature_group WHERE name = 'Redump'",
	)
	.await
	.expect("the Redump signature group is inserted by an earlier migration")
	.to_string()
}

async fn seed_private_merge_scenario(db: &DbConn, sg: &str) {
	let sql = format!(
		r#"
		INSERT INTO platform (id, name) VALUES ('{M_PLAT}', 'Wii U');
		INSERT INTO platform (id, name) VALUES ('{M_PLAT2}', 'Xbox 360');
		INSERT INTO platform (id, name) VALUES ('{M_PLAT3}', 'PlayStation 3');

		INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id, latest_dat_file_import_id)
		VALUES ('{PUB_DF}', 'Nintendo - Wii U', '{M_PLAT}', '2026-07-01', '{sg}', '{I_PUB}');
		INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id, latest_dat_file_import_id)
		VALUES ('{PRIV_DF}', 'Nintendo - Wii U (2025-07-19 10-00-00)', '{M_PLAT}', '2025-07-19', '{sg}', '{I_PRIV}');
		INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id, latest_dat_file_import_id)
		VALUES ('{ORPH_DF}', 'Microsoft_-_Xbox_360_-_2025_07_19', '{M_PLAT2}', '2025-07-19', '{sg}', '{I_ORPH}');
		INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id, latest_dat_file_import_id)
		VALUES ('{MERGED_DF}', 'Sony - PlayStation 3', '{M_PLAT3}', '2026-07-01', '{sg}', '{I_MERGED}');

		INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at)
		VALUES ('{I_PUB}', '{PUB_DF}', 'Nintendo - Wii U.dat', '2026-07-01', 'pubpubpub', '2026-07-01 12:00:00+00');
		INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at)
		VALUES ('{I_PRIV}', '{PRIV_DF}', 'Nintendo - Wii U (2025-07-19 10-00-00).dat', '2025-07-19', 'privpriv', '2025-07-19 12:00:00+00');
		INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at)
		VALUES ('{I_ORPH}', '{ORPH_DF}', 'Microsoft_-_Xbox_360_-_2025_07_19.dat', '2025-07-19', 'orphorph', '2025-07-19 12:00:00+00');
		INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at)
		VALUES ('{I_MERGED}', '{MERGED_DF}', 'Sony - PlayStation 3.dat', '2026-07-01', 'mrgmrgmr', '2026-07-01 12:00:00+00');

		INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id)
		VALUES ('{PUB_GAME}', '{I_PUB}', 'Some Wii U Game (USA)', true, '{I_PUB}');
		INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id)
		VALUES ('{PRIV_GAME}', '{I_PRIV}', 'Some Wii U Game (USA)', true, '{I_PRIV}');
		INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id)
		VALUES ('{ORPH_GAME}', '{I_ORPH}', 'Some Xbox Game (USA)', true, '{I_ORPH}');
		INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id)
		VALUES ('{MERGED_GAME}', '{I_MERGED}', 'Some PS3 Game (USA)', true, '{I_MERGED}');

		INSERT INTO game_file (id, game_id, file_name, sha1, is_current, last_seen_dat_file_import_id)
		VALUES ('{PUB_GF}', '{PUB_GAME}', 'Some Wii U Game (USA).iso', 'wiiupub1', true, '{I_PUB}');
		INSERT INTO game_file (id, game_id, file_name, sha1, is_current, last_seen_dat_file_import_id)
		VALUES ('{PRIV_GF}', '{PRIV_GAME}', 'Some Wii U Game (USA).iso', 'wiiupriv', true, '{I_PRIV}');
		INSERT INTO game_file (id, game_id, file_name, sha1, is_current, last_seen_dat_file_import_id)
		VALUES ('{ORPH_GF}', '{ORPH_GAME}', 'Some Xbox Game (USA).iso', 'xbox0001', true, '{I_ORPH}');
		INSERT INTO game_file (id, game_id, file_name, sha1, is_current, last_seen_dat_file_import_id)
		VALUES ('{MERGED_GF}', '{MERGED_GAME}', 'Some PS3 Game (USA).iso', 'ps3mrg01', true, '{I_MERGED}');
		"#
	);
	db.execute_unprepared(&sql).await.unwrap();
}

async fn count_named(db: &DbConn, name: &str) -> i64 {
	let row = db
		.query_one(Statement::from_string(
			db.get_database_backend(),
			format!("SELECT count(*) AS cnt FROM dat_file WHERE name = '{name}'"),
		))
		.await
		.unwrap()
		.unwrap();
	row.try_get::<i64>("", "cnt").unwrap()
}

async fn dat_file_name(db: &DbConn, id: &str) -> Option<String> {
	db.query_one(Statement::from_string(
		db.get_database_backend(),
		format!("SELECT name FROM dat_file WHERE id = '{id}'"),
	))
	.await
	.unwrap()
	.map(|row| row.try_get::<String>("", "name").unwrap())
}

async fn scalar_uuid(db: &DbConn, sql: &str) -> Option<Uuid> {
	db.query_one(Statement::from_string(
		db.get_database_backend(),
		sql.to_owned(),
	))
	.await
	.unwrap()
	.and_then(|row| row.try_get::<Option<Uuid>>("", "v").unwrap())
}
