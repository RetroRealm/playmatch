//! Postgres-backed tests for the v2 stats service queries.
//! Require Docker.

use migration::{Migrator, MigratorTrait};
use sea_orm::prelude::Uuid;
use sea_orm::{ConnectionTrait, Database, DbConn};
use service::db::stats::{collect_global_counts, platform_exists};
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use testcontainers_modules::testcontainers::{ContainerAsync, ImageExt};

/// The returned container guard must be kept alive for the test's duration.
async fn start_pg() -> (ContainerAsync<Postgres>, DbConn) {
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

const SG_ID: &str = "11111111-1111-1111-1111-111111111111";
const PLATFORM: &str = "22222222-2222-2222-2222-222222222222";
const DAT: &str = "33333333-3333-3333-3333-333333333333";
const IMPORT: &str = "44444444-4444-4444-4444-444444444444";

#[tokio::test]
async fn global_counts_are_zero_on_an_empty_database() {
	let (_pg, db) = start_pg().await;

	let counts = collect_global_counts(&db).await.unwrap();

	assert_eq!(counts.game_count, 0);
	assert_eq!(counts.current_game_count, 0);
	assert_eq!(counts.game_file_count, 0);
	assert_eq!(counts.dat_file_count, 0);
	assert_eq!(counts.mapped_game_count, 0);
	assert!(
		counts.last_import_at.is_none(),
		"no imports means no latest import time"
	);

	// Migrations seed five reference signature groups (No-Intro, Redump, TOSEC,
	// MAME, DatsSite-Legacy); none are user data.
	assert_eq!(counts.signature_group_count, 5);
}

#[tokio::test]
async fn mapped_game_count_only_counts_successfully_mapped_games() {
	let (_pg, db) = start_pg().await;
	seed_mixed_mapping(&db).await;

	let counts = collect_global_counts(&db).await.unwrap();

	assert_eq!(counts.game_count, 4, "every seeded game is counted");
	assert_eq!(
		counts.mapped_game_count, 2,
		"only the automatic and manual matches count as mapped"
	);
}

#[tokio::test]
async fn platform_exists_is_false_for_an_unknown_platform() {
	let (_pg, db) = start_pg().await;

	let unknown = Uuid::parse_str("99999999-9999-9999-9999-999999999999").unwrap();
	assert!(!platform_exists(unknown, &db).await.unwrap());
}

/// Four games on one dat import. Two carry a successful mapping (one automatic,
/// one manual), one carries a failed mapping and one has none at all, so the
/// mapped count must come out to exactly two.
async fn seed_mixed_mapping(db: &DbConn) {
	let game_mapped_auto = "a1a1a1a1-a1a1-a1a1-a1a1-a1a1a1a1a1a1";
	let game_mapped_manual = "b2b2b2b2-b2b2-b2b2-b2b2-b2b2b2b2b2b2";
	let game_failed = "c3c3c3c3-c3c3-c3c3-c3c3-c3c3c3c3c3c3";
	let game_unmapped = "d4d4d4d4-d4d4-d4d4-d4d4-d4d4d4d4d4d4";

	let sql = format!(
		r#"
		INSERT INTO signature_group (id, name) VALUES ('{SG_ID}', 'No-Intro');
		INSERT INTO platform (id, name) VALUES ('{PLATFORM}', 'plat-a');
		INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id, latest_dat_file_import_id)
		VALUES ('{DAT}', 'dat-a', '{PLATFORM}', '1.0', '{SG_ID}', '{IMPORT}');
		INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at)
		VALUES ('{IMPORT}', '{DAT}', 'dat-a (2024)', '1.0', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', '2024-01-01T00:00:00Z');
		INSERT INTO game (id, dat_file_import_id, name, is_current) VALUES
		  ('{game_mapped_auto}', '{IMPORT}', 'Auto Game', true),
		  ('{game_mapped_manual}', '{IMPORT}', 'Manual Game', true),
		  ('{game_failed}', '{IMPORT}', 'Failed Game', true),
		  ('{game_unmapped}', '{IMPORT}', 'Unmapped Game', true);
		INSERT INTO signature_metadata_mapping
		  (id, game_id, provider, provider_id, match_type, automatic_match_reason, matched_name)
		VALUES
		  (gen_random_uuid(), '{game_mapped_auto}', 'igdb', 'igdb-1', 'automatic', 'direct_name', 'Auto Game'),
		  (gen_random_uuid(), '{game_mapped_manual}', 'igdb', 'igdb-2', 'manual', NULL, 'Manual Game'),
		  (gen_random_uuid(), '{game_failed}', 'igdb', NULL, 'failed', NULL, NULL);
		"#
	);
	db.execute_unprepared(&sql).await.unwrap();
}
