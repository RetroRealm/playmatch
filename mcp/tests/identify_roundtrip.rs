//! Postgres + Redis backed round-trip tests for the MCP tool free functions.
//! Require Docker; ignored by default. Run with:
//!   cargo test -p mcp --test identify_roundtrip -- --ignored

use migration::{Migrator, MigratorTrait};
use redis::aio::MultiplexedConnection;
use sea_orm::{ConnectionTrait, Database, DbConn};
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::redis::{REDIS_PORT, Redis};
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use testcontainers_modules::testcontainers::{ContainerAsync, ImageExt};

const SG: &str = "11111111-1111-1111-1111-111111111111";
const PLAT: &str = "22222222-2222-2222-2222-222222222222";
const DF: &str = "a1a1a1a1-a1a1-a1a1-a1a1-a1a1a1a1a1a1";
const IMPORT: &str = "c3c3c3c3-c3c3-c3c3-c3c3-c3c3c3c3c3c3";
const GAME: &str = "e5e5e5e5-e5e5-e5e5-e5e5-e5e5e5e5e5e5";
const GF: &str = "99999999-9999-9999-9999-999999999999";
const SHA1: &str = "432dbe312bc51e36bb8cb6fcb5e08f6968f124a4";

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

async fn seed_game(db: &DbConn) {
	let sql = format!(
		r#"
		INSERT INTO signature_group (id, name) VALUES ('{SG}', 'No-Intro');
		INSERT INTO platform (id, name) VALUES ('{PLAT}', 'Nintendo DS');

		INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id, latest_dat_file_import_id)
		VALUES ('{DF}', 'Nintendo - Nintendo DS (Decrypted)', '{PLAT}', '20260617-122122', '{SG}', '{IMPORT}');

		INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at)
		VALUES ('{IMPORT}', '{DF}', 'Nintendo - Nintendo DS (Decrypted) (20260617-122122).dat', '20260617-122122', 'aaaaaaaa', '2026-06-18 12:00:00+00');

		INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id)
		VALUES ('{GAME}', '{IMPORT}', 'Pokemon - Diamant-Edition (Germany) (Rev 5)', true, '{IMPORT}');

		INSERT INTO game_file (id, game_id, file_name, sha1, is_current, last_seen_dat_file_import_id)
		VALUES ('{GF}', '{GAME}', 'Pokemon - Diamant-Edition (Germany) (Rev 5).nds', '{SHA1}', true, '{IMPORT}');
		"#
	);
	db.execute_unprepared(&sql).await.unwrap();
}

#[tokio::test]
#[ignore = "requires Docker (testcontainers Postgres + Redis)"]
async fn identify_by_hash_and_get_game_return_seeded_game() {
	let (_pg, db) = start_pg().await;
	let (_redis, mut redis) = start_redis().await;
	seed_game(&db).await;

	let search = mcp::tools::build_search(
		"Pokemon - Diamant-Edition (Germany) (Rev 5).nds".to_string(),
		1024,
		None,
		Some(SHA1.to_string()),
		None,
	);
	let json = mcp::tools::identify_rom_by_hash_json(search, &mut redis, &db)
		.await
		.unwrap();
	assert!(
		json.contains("SHA1"),
		"a sha1 hit must report a SHA1 match type, got: {json}"
	);
	assert!(
		json.contains(GAME),
		"the matched payload must carry the seeded game id, got: {json}"
	);

	let game_id = sea_orm::prelude::Uuid::parse_str(GAME).unwrap();
	let game_json = mcp::tools::get_game_json(game_id, &db)
		.await
		.unwrap()
		.expect("seeded game must be found");
	assert!(
		game_json.contains("Pokemon - Diamant-Edition (Germany) (Rev 5)"),
		"get_game must return the seeded game name, got: {game_json}"
	);

	let unknown =
		sea_orm::prelude::Uuid::parse_str("dddddddd-dddd-dddd-dddd-dddddddddddd").unwrap();
	assert!(
		mcp::tools::get_game_json(unknown, &db)
			.await
			.unwrap()
			.is_none(),
		"an unknown game id must resolve to None, not an error"
	);
}

#[tokio::test]
#[ignore = "requires Docker (testcontainers Postgres + Redis)"]
async fn identify_unknown_hash_returns_nomatch() {
	let (_pg, db) = start_pg().await;
	let (_redis, mut redis) = start_redis().await;

	let search = mcp::tools::build_search(
		"unknown.rom".to_string(),
		2048,
		None,
		Some("0000000000000000000000000000000000000000".to_string()),
		None,
	);
	let json = mcp::tools::identify_rom_by_hash_json(search, &mut redis, &db)
		.await
		.unwrap();
	assert!(
		json.contains("NoMatch"),
		"an unknown hash must be a normal NoMatch result, got: {json}"
	);
}
