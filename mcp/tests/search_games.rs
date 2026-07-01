//! Postgres backed round-trip test for the fuzzy game-name search tool.
//! Requires Docker.

use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DbConn};
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use testcontainers_modules::testcontainers::{ContainerAsync, ImageExt};

const SG: &str = "11111111-1111-1111-1111-111111111111";
const PLAT_DS: &str = "22222222-2222-2222-2222-222222222222";
const PLAT_GBA: &str = "33333333-3333-3333-3333-333333333333";
const DF_DS: &str = "a1a1a1a1-a1a1-a1a1-a1a1-a1a1a1a1a1a1";
const DF_GBA: &str = "b2b2b2b2-b2b2-b2b2-b2b2-b2b2b2b2b2b2";
const IMPORT_DS: &str = "c3c3c3c3-c3c3-c3c3-c3c3-c3c3c3c3c3c3";
const IMPORT_GBA: &str = "d4d4d4d4-d4d4-d4d4-d4d4-d4d4d4d4d4d4";
const GAME_DIAMOND: &str = "e5e5e5e5-e5e5-e5e5-e5e5-e5e5e5e5e5e5";
const GAME_PINBALL: &str = "f6f6f6f6-f6f6-f6f6-f6f6-f6f6f6f6f6f6";

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

async fn seed_games(db: &DbConn) {
	let sql = format!(
		r#"
		INSERT INTO signature_group (id, name) VALUES ('{SG}', 'No-Intro');
		INSERT INTO platform (id, name) VALUES ('{PLAT_DS}', 'Nintendo DS');
		INSERT INTO platform (id, name) VALUES ('{PLAT_GBA}', 'Game Boy Advance');

		INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id, latest_dat_file_import_id)
		VALUES ('{DF_DS}', 'Nintendo - Nintendo DS (Decrypted)', '{PLAT_DS}', '20260617-122122', '{SG}', '{IMPORT_DS}');
		INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id, latest_dat_file_import_id)
		VALUES ('{DF_GBA}', 'Nintendo - Game Boy Advance', '{PLAT_GBA}', '20260617-122122', '{SG}', '{IMPORT_GBA}');

		INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at)
		VALUES ('{IMPORT_DS}', '{DF_DS}', 'Nintendo - Nintendo DS (Decrypted) (20260617-122122).dat', '20260617-122122', 'aaaaaaaa', '2026-06-18 12:00:00+00');
		INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at)
		VALUES ('{IMPORT_GBA}', '{DF_GBA}', 'Nintendo - Game Boy Advance (20260617-122122).dat', '20260617-122122', 'bbbbbbbb', '2026-06-18 12:00:00+00');

		INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id)
		VALUES ('{GAME_DIAMOND}', '{IMPORT_DS}', 'Pokemon - Diamant-Edition (Germany) (Rev 5)', true, '{IMPORT_DS}');
		INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id)
		VALUES ('{GAME_PINBALL}', '{IMPORT_GBA}', 'Pokemon Pinball - Ruby & Sapphire (Europe)', true, '{IMPORT_GBA}');
		"#
	);
	db.execute_unprepared(&sql).await.unwrap();
}

#[tokio::test]
async fn fuzzy_search_returns_canonical_pokemon_diamond() {
	let (_pg, db) = start_pg().await;
	seed_games(&db).await;

	let json = mcp::tools::search_games_by_name_json("pokemon diamond", None, None, &db)
		.await
		.unwrap();

	assert!(
		json.contains(GAME_DIAMOND),
		"a fuzzy 'pokemon diamond' search must return the Diamant row id, got: {json}"
	);
	assert!(
		json.contains("Pokemon - Diamant-Edition (Germany) (Rev 5)"),
		"the matched candidate must carry the canonical game name, got: {json}"
	);
	assert!(
		json.contains("Nintendo DS"),
		"each candidate must carry its platform name, got: {json}"
	);
}

#[tokio::test]
async fn fuzzy_search_respects_platform_filter() {
	let (_pg, db) = start_pg().await;
	seed_games(&db).await;

	let gba = sea_orm::prelude::Uuid::parse_str(PLAT_GBA).unwrap();
	let json = mcp::tools::search_games_by_name_json("pokemon", Some(gba), None, &db)
		.await
		.unwrap();

	assert!(
		json.contains(GAME_PINBALL),
		"the Game Boy Advance Pokemon title must be returned, got: {json}"
	);
	assert!(
		!json.contains(GAME_DIAMOND),
		"the platform filter must exclude the Nintendo DS title, got: {json}"
	);
}
