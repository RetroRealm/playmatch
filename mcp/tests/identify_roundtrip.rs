//! Postgres + Redis backed round-trip tests for the MCP tool free functions.
//! Require Docker.

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
const CRC: &str = "1a2b3c4d";

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

		INSERT INTO game_file (id, game_id, file_name, file_size_in_bytes, sha1, crc, is_current, last_seen_dat_file_import_id)
		VALUES ('{GF}', '{GAME}', 'Pokemon - Diamant-Edition (Germany) (Rev 5).nds', 1024, '{SHA1}', '{CRC}', true, '{IMPORT}');

		INSERT INTO game_file_presence (game_file_id, dat_file_import_id)
		VALUES ('{GF}', '{IMPORT}');
		"#
	);
	db.execute_unprepared(&sql).await.unwrap();
}

#[tokio::test]
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

	let crc_search = mcp::tools::build_search(
		"Pokemon - Diamant-Edition (Germany) (Rev 5).nds".to_string(),
		1024,
		None,
		None,
		None,
		Some(CRC.to_string()),
	);
	let crc_json = mcp::tools::identify_rom_by_hash_json(crc_search, &mut redis, &db)
		.await
		.unwrap();
	assert!(
		crc_json.contains("CRC"),
		"a crc hit must report a CRC match type, got: {crc_json}"
	);
	assert!(
		crc_json.contains(GAME),
		"the crc-matched payload must carry the seeded game id, got: {crc_json}"
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
async fn identify_unknown_hash_returns_nomatch() {
	let (_pg, db) = start_pg().await;
	let (_redis, mut redis) = start_redis().await;

	let search = mcp::tools::build_search(
		"unknown.rom".to_string(),
		2048,
		None,
		Some("0000000000000000000000000000000000000000".to_string()),
		None,
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

#[tokio::test]
async fn dat_file_and_game_file_tools_return_seeded_records() {
	let (_pg, db) = start_pg().await;
	seed_game(&db).await;

	let dat_file_id = sea_orm::prelude::Uuid::parse_str(DF).unwrap();
	let game_id = sea_orm::prelude::Uuid::parse_str(GAME).unwrap();

	let list = mcp::tools::list_dat_files_json(None, None, None, None, None, &db)
		.await
		.unwrap();
	assert!(
		list.contains(DF),
		"the dat file listing must carry the seeded dat file id, got: {list}"
	);

	let detail = mcp::tools::get_dat_file_json(dat_file_id, &db)
		.await
		.unwrap()
		.expect("seeded dat file must be found");
	assert!(
		detail.contains("gameCount"),
		"the dat file detail must carry aggregate game counts, got: {detail}"
	);

	let games = mcp::tools::list_dat_file_games_json(dat_file_id, true, false, false, None, &db)
		.await
		.unwrap()
		.expect("seeded dat file must resolve its games");
	assert!(
		games.contains("Pokemon - Diamant-Edition (Germany) (Rev 5)"),
		"the dat file games must carry the seeded game name, got: {games}"
	);

	let files = mcp::tools::get_game_files_json(game_id, true, None, &db)
		.await
		.unwrap()
		.expect("seeded game must resolve its files");
	assert!(
		files.contains(SHA1),
		"the game files must carry the seeded sha1, got: {files}"
	);

	let unknown =
		sea_orm::prelude::Uuid::parse_str("dddddddd-dddd-dddd-dddd-dddddddddddd").unwrap();
	assert!(
		mcp::tools::get_dat_file_json(unknown, &db)
			.await
			.unwrap()
			.is_none(),
		"an unknown dat file id must resolve to None"
	);
	assert!(
		mcp::tools::get_game_files_json(unknown, true, None, &db)
			.await
			.unwrap()
			.is_none(),
		"an unknown game id must resolve to None for the files tool"
	);
}

#[tokio::test]
async fn reverse_hash_lookup_and_bulk_identify_match_the_single_endpoint() {
	let (_pg, db) = start_pg().await;
	let (_redis, redis) = start_redis().await;
	seed_game(&db).await;

	let lookup = service::entities::dat_file::HashLookup {
		sha1: Some(SHA1.to_string()),
		..Default::default()
	};
	let presence = mcp::tools::find_dats_containing_hash_json(lookup, false, &db)
		.await
		.unwrap()
		.expect("a seeded sha1 must resolve to a dat file presence");
	assert!(
		presence.contains(DF),
		"the reverse lookup must carry the seeded dat file id, got: {presence}"
	);

	let items = vec![
		mcp::tools::BulkIdentifyItem {
			search: mcp::tools::build_search(
				"Pokemon - Diamant-Edition (Germany) (Rev 5).nds".to_string(),
				1024,
				None,
				Some(SHA1.to_string()),
				None,
				None,
			),
			key: Some("hit".to_string()),
		},
		mcp::tools::BulkIdentifyItem {
			search: mcp::tools::build_search(
				"unknown.rom".to_string(),
				2048,
				None,
				Some("0000000000000000000000000000000000000000".to_string()),
				None,
				None,
			),
			key: None,
		},
	];

	let json = mcp::tools::bulk_identify_json(items, &redis, &db)
		.await
		.unwrap();
	assert!(
		json.contains("\"total\":2"),
		"the bulk summary must report two items, got: {json}"
	);
	assert!(
		json.contains(GAME) && json.contains("SHA1"),
		"the bulk hit must mirror the single endpoint's SHA1 match, got: {json}"
	);
	assert!(
		json.contains("NoMatch") && json.contains("\"hit\""),
		"the bulk batch must carry both the no-match item and the echoed key, got: {json}"
	);
}
