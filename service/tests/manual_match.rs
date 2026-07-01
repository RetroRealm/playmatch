//! Postgres + Redis backed tests for manual game matching fan-out and idempotency.
//! Require Docker.

use migration::{Migrator, MigratorTrait};
use redis::aio::MultiplexedConnection;
use sea_orm::prelude::Uuid;
use sea_orm::{ConnectionTrait, Database, DbConn, Statement};
use service::matching::manual::apply_manual_game_match;
use service::model::MetadataProvider;
use service::model::matching::GameMatchRequest;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::redis::{REDIS_PORT, Redis};
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use testcontainers_modules::testcontainers::{ContainerAsync, ImageExt};

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

async fn start_redis() -> (ContainerAsync<Redis>, MultiplexedConnection) {
	let container = Redis::default().start().await.unwrap();
	let port = container.get_host_port_ipv4(REDIS_PORT).await.unwrap();
	let url = format!("redis://127.0.0.1:{port}");
	let client = redis::Client::open(url).unwrap();
	let conn = client.get_multiplexed_async_connection().await.unwrap();
	(container, conn)
}

const SG: &str = "11111111-1111-1111-1111-111111111111";
const PLAT: &str = "22222222-2222-2222-2222-222222222222";
const DF: &str = "a1a1a1a1-a1a1-a1a1-a1a1-a1a1a1a1a1a1";
const IMPORT: &str = "c3c3c3c3-c3c3-c3c3-c3c3-c3c3c3c3c3c3";

const PARENT: &str = "e0e0e0e0-e0e0-e0e0-e0e0-e0e0e0e0e0e0";
const CLONE_A: &str = "e1e1e1e1-e1e1-e1e1-e1e1-e1e1e1e1e1e1";
const CLONE_B: &str = "e2e2e2e2-e2e2-e2e2-e2e2-e2e2e2e2e2e2";
const SIBLING: &str = "e3e3e3e3-e3e3-e3e3-e3e3-e3e3e3e3e3e3";

const GF_CLONE_A: &str = "f1f1f1f1-f1f1-f1f1-f1f1-f1f1f1f1f1f1";
const TARGET_SHA1: &str = "432dbe312bc51e36bb8cb6fcb5e08f6968f124a4";

const SHARED_NAME: &str = "Pokemon - Diamant-Edition (Germany)";
const PROVIDER_ID: &str = "9999";

// The parent, its two clones, and a same-name same-platform top-level sibling
// all share one game name so the name+platform fan-out reaches every row, then
// the parent/children expansion keeps the same set.
async fn seed_family(db: &DbConn) {
	let sql = format!(
		r#"
		INSERT INTO signature_group (id, name) VALUES ('{SG}', 'No-Intro');
		INSERT INTO platform (id, name) VALUES ('{PLAT}', 'Nintendo DS');

		INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id, latest_dat_file_import_id)
		VALUES ('{DF}', 'Nintendo - Nintendo DS', '{PLAT}', '20260617-122122', '{SG}', '{IMPORT}');

		INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at)
		VALUES ('{IMPORT}', '{DF}', 'Nintendo - Nintendo DS (20260617-122122).dat', '20260617-122122', 'aaaaaaaa', '2026-06-18 12:00:00+00');

		INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id, clone_of)
		VALUES ('{PARENT}', '{IMPORT}', '{SHARED_NAME}', true, '{IMPORT}', NULL);
		INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id, clone_of)
		VALUES ('{CLONE_A}', '{IMPORT}', '{SHARED_NAME}', true, '{IMPORT}', '{PARENT}');
		INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id, clone_of)
		VALUES ('{CLONE_B}', '{IMPORT}', '{SHARED_NAME}', true, '{IMPORT}', '{PARENT}');
		INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id, clone_of)
		VALUES ('{SIBLING}', '{IMPORT}', '{SHARED_NAME}', true, '{IMPORT}', NULL);

		INSERT INTO game_file (id, game_id, file_name, sha1, is_current, last_seen_dat_file_import_id)
		VALUES ('{GF_CLONE_A}', '{CLONE_A}', '{SHARED_NAME} (Rev 1).nds', '{TARGET_SHA1}', true, '{IMPORT}');
		"#
	);
	db.execute_unprepared(&sql).await.unwrap();
}

fn match_request() -> GameMatchRequest {
	GameMatchRequest {
		md5: None,
		sha1: Some(TARGET_SHA1.to_string()),
		sha256: None,
		name: None,
		comment: None,
		provider: MetadataProvider::IGDB,
		provider_id: PROVIDER_ID.to_string(),
		manual_match_type: service::model::ManualMatchMode::Admin,
		user_id: None,
		matched_name: None,
	}
}

async fn mapped_game_ids(db: &DbConn) -> std::collections::HashSet<String> {
	let backend = db.get_database_backend();
	let rows = db
		.query_all(Statement::from_string(
			backend,
			format!(
				r#"
				SELECT game_id::text AS gid
				FROM signature_metadata_mapping
				WHERE provider = 'igdb'
				  AND provider_id = '{PROVIDER_ID}'
				  AND match_type = 'manual'
				  AND game_id IS NOT NULL
				"#
			),
		))
		.await
		.unwrap();
	rows.into_iter()
		.map(|row| row.try_get::<String>("", "gid").unwrap())
		.collect()
}

#[tokio::test]
async fn manual_match_fans_out_to_parent_clones_and_same_name_sibling() {
	let (_pg, db) = start_pg().await;
	let (_redis, mut redis) = start_redis().await;
	seed_family(&db).await;

	let results = apply_manual_game_match(match_request(), &db, &mut redis)
		.await
		.unwrap();

	let mapped = mapped_game_ids(&db).await;

	let expected: std::collections::HashSet<String> = [PARENT, CLONE_A, CLONE_B, SIBLING]
		.iter()
		.map(|s| s.to_string())
		.collect();

	assert_eq!(
		mapped, expected,
		"manual match by a clone's hash must write a Manual mapping for the parent, both clones, and the same-name sibling"
	);

	assert_eq!(
		results.len(),
		4,
		"manual match must return exactly four results with no duplicate UpdatedMatchResult rows, got {}",
		results.len()
	);

	let distinct_updated: std::collections::HashSet<Uuid> = results.iter().map(|r| r.id).collect();
	assert_eq!(
		distinct_updated,
		expected
			.iter()
			.map(|s| Uuid::parse_str(s).unwrap())
			.collect(),
		"the returned results must cover exactly the four related games"
	);
}

#[tokio::test]
async fn manual_match_is_idempotent_on_repeat() {
	let (_pg, db) = start_pg().await;
	let (_redis, mut redis) = start_redis().await;
	seed_family(&db).await;

	apply_manual_game_match(match_request(), &db, &mut redis)
		.await
		.unwrap();
	let after_first = mapped_game_ids(&db).await;

	let second = apply_manual_game_match(match_request(), &db, &mut redis)
		.await
		.unwrap();

	assert!(
		second.is_empty(),
		"a second identical manual match must update zero games, got {} results",
		second.len()
	);

	let after_second = mapped_game_ids(&db).await;
	assert_eq!(
		after_first, after_second,
		"the mapping set must be unchanged after an identical repeat call"
	);
}
