//! Postgres-backed tests for the v2 stats, presence and signature-group-games
//! endpoints: service-wide stats, per-platform stats, the game-file presence
//! changelog and the signature-group games listing. They drive the real
//! actix v2 scope end to end so routing precedence, the count aggregation
//! and the 404 paths are all exercised.
//! Require Docker.

use actix_web::web::Data;
use actix_web::{App, test};
use api::{
	PublicRouteFlags, public_api_governor_config, test_prometheus_metrics, versioned_api_scope,
};
use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};
use serde_json::Value;
use service::config::versions::ApiVersion;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use testcontainers_modules::testcontainers::{ContainerAsync, ImageExt};

const SG_ID: &str = "11111111-1111-1111-1111-111111111111";
const PLATFORM_A: &str = "22222222-2222-2222-2222-222222222222";
const PLATFORM_B: &str = "2b2b2b2b-2b2b-2b2b-2b2b-2b2b2b2b2b2b";
const DAT_A: &str = "33333333-3333-3333-3333-333333333333";
const IMPORT_1: &str = "44444444-4444-4444-4444-444444444444";
const IMPORT_2: &str = "4b4b4b4b-4b4b-4b4b-4b4b-4b4b4b4b4b4b";
const GAME_A: &str = "55555555-5555-5555-5555-555555555555";
const GAME_B: &str = "66666666-6666-6666-6666-666666666666";
const GAME_FILE: &str = "77777777-7777-7777-7777-777777777777";

async fn start_pg() -> (ContainerAsync<Postgres>, DatabaseConnection) {
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

/// One signature group publishing one dat file on platform A with two imports.
/// Two current games, one carrying an automatic mapping (so it is "mapped"); one
/// game file observed in both imports so the presence changelog has two rows.
async fn seed(db: &DatabaseConnection) {
	let sql = format!(
		r#"
        INSERT INTO signature_group (id, name) VALUES ('{SG_ID}', 'No-Intro');
        INSERT INTO platform (id, name) VALUES
          ('{PLATFORM_A}', 'plat-a'),
          ('{PLATFORM_B}', 'plat-b');
        INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id, latest_dat_file_import_id) VALUES
          ('{DAT_A}', 'dat-a', '{PLATFORM_A}', '2.0', '{SG_ID}', '{IMPORT_2}');
        INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at) VALUES
          ('{IMPORT_1}', '{DAT_A}', 'dat-a (2023)', '1.0', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', '2023-01-01T00:00:00Z'),
          ('{IMPORT_2}', '{DAT_A}', 'dat-a (2024)', '2.0', 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb', '2024-01-01T00:00:00Z');
        INSERT INTO game (id, dat_file_import_id, name, is_current) VALUES
          ('{GAME_A}', '{IMPORT_2}', 'Alpha Game', true),
          ('{GAME_B}', '{IMPORT_2}', 'Beta Game', true);
        INSERT INTO game_file (id, game_id, file_name, is_current) VALUES
          ('{GAME_FILE}', '{GAME_A}', 'alpha.rom', true);
        INSERT INTO game_file_presence (id, game_file_id, dat_file_import_id) VALUES
          (gen_random_uuid(), '{GAME_FILE}', '{IMPORT_1}'),
          (gen_random_uuid(), '{GAME_FILE}', '{IMPORT_2}');
        INSERT INTO signature_metadata_mapping
          (id, game_id, provider, provider_id, match_type, automatic_match_reason, matched_name)
        VALUES
          (gen_random_uuid(), '{GAME_A}', 'igdb', 'igdb-1', 'automatic', 'direct_name', 'Alpha Game');
    "#
	);
	db.execute_unprepared(&sql).await.unwrap();
}

macro_rules! build_app {
	($db:expr) => {{
		let gov = public_api_governor_config();
		let prom = test_prometheus_metrics();
		App::new()
			.app_data(Data::new($db))
			.service(versioned_api_scope(
				"/api/v2",
				ApiVersion::V2,
				PublicRouteFlags::default(),
				&gov,
				prom,
			))
	}};
}

macro_rules! get_json {
	($app:expr, $uri:expr) => {{
		let req = test::TestRequest::get()
			.uri($uri)
			.peer_addr("203.0.113.9:0".parse().unwrap())
			.to_request();
		let resp = test::call_service(&$app, req).await;
		let status = resp.status().as_u16();
		let body = test::read_body(resp).await;
		let json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
		(status, json)
	}};
}

#[actix_web::test]
async fn service_stats_aggregates_every_table() {
	let (_pg, db) = start_pg().await;
	seed(&db).await;
	let app = test::init_service(build_app!(db)).await;

	let (status, stats) = get_json!(app, "/api/v2/stats");
	assert_eq!(status, 200);
	assert_eq!(stats["datFileCount"], 1);
	// Migrations seed 5 reference signature groups (No-Intro, Redump, TOSEC,
	// MAME, DatsSite-Legacy); the test seed adds one more.
	assert_eq!(stats["signatureGroupCount"], 6);
	assert_eq!(stats["platformCount"], 2);
	assert_eq!(stats["gameCount"], 2);
	assert_eq!(stats["currentGameCount"], 2);
	assert_eq!(stats["gameFileCount"], 1);
	assert_eq!(stats["mappedGameCount"], 1);
	assert!(stats["lastImportAt"].as_str().unwrap().starts_with("2024"));
}

#[actix_web::test]
async fn platform_stats_scopes_to_the_platform() {
	let (_pg, db) = start_pg().await;
	seed(&db).await;
	let app = test::init_service(build_app!(db)).await;

	let (status, stats) = get_json!(app, &format!("/api/v2/platforms/{PLATFORM_A}/stats"));
	assert_eq!(status, 200);
	assert_eq!(stats["datFileCount"], 1);
	assert_eq!(stats["currentGameCount"], 2);
	assert_eq!(stats["gameFileCount"], 1);
	assert_eq!(stats["mappedGameCount"], 1);

	let (status_b, stats_b) = get_json!(app, &format!("/api/v2/platforms/{PLATFORM_B}/stats"));
	assert_eq!(status_b, 200);
	assert_eq!(stats_b["datFileCount"], 0);
	assert_eq!(stats_b["currentGameCount"], 0);
}

#[actix_web::test]
async fn platform_stats_unknown_platform_is_404() {
	let (_pg, db) = start_pg().await;
	let app = test::init_service(build_app!(db)).await;

	let (status, _) = get_json!(
		app,
		"/api/v2/platforms/99999999-9999-9999-9999-999999999999/stats"
	);
	assert_eq!(status, 404);
}

#[actix_web::test]
async fn presence_changelog_lists_every_observation_newest_first() {
	let (_pg, db) = start_pg().await;
	seed(&db).await;
	let app = test::init_service(build_app!(db)).await;

	let (status, entries) = get_json!(app, &format!("/api/v2/game-files/{GAME_FILE}/presence"));
	assert_eq!(status, 200);
	let arr = entries.as_array().unwrap();
	assert_eq!(arr.len(), 2);
	assert_eq!(arr[0]["version"], "2.0");
	assert_eq!(arr[1]["version"], "1.0");
	assert_eq!(arr[0]["datFileId"], DAT_A);
	assert_eq!(arr[0]["platformId"], PLATFORM_A);
}

#[actix_web::test]
async fn presence_changelog_unknown_game_file_is_404() {
	let (_pg, db) = start_pg().await;
	let app = test::init_service(build_app!(db)).await;

	let (status, _) = get_json!(
		app,
		"/api/v2/game-files/99999999-9999-9999-9999-999999999999/presence"
	);
	assert_eq!(status, 404);
}

#[actix_web::test]
async fn signature_group_games_lists_current_games_ordered_by_name() {
	let (_pg, db) = start_pg().await;
	seed(&db).await;
	let app = test::init_service(build_app!(db)).await;

	let (status, page) = get_json!(app, &format!("/api/v2/signature-groups/{SG_ID}/games"));
	assert_eq!(status, 200);
	let names: Vec<String> = page["data"]
		.as_array()
		.unwrap()
		.iter()
		.map(|g| g["name"].as_str().unwrap().to_string())
		.collect();
	assert_eq!(names, vec!["Alpha Game", "Beta Game"]);
}

#[actix_web::test]
async fn signature_group_games_platform_filter_and_pagination() {
	let (_pg, db) = start_pg().await;
	seed(&db).await;
	let app = test::init_service(build_app!(db)).await;

	let (_, empty) = get_json!(
		app,
		&format!("/api/v2/signature-groups/{SG_ID}/games?platformId={PLATFORM_B}")
	);
	assert!(empty["data"].as_array().unwrap().is_empty());

	let (_, page) = get_json!(
		app,
		&format!("/api/v2/signature-groups/{SG_ID}/games?limit=1")
	);
	assert_eq!(page["data"].as_array().unwrap().len(), 1);
	assert_eq!(page["pagination"]["hasNextPage"], true);
	let cursor = page["pagination"]["nextCursor"].as_str().unwrap();

	let (_, page2) = get_json!(
		app,
		&format!("/api/v2/signature-groups/{SG_ID}/games?limit=1&cursor={cursor}")
	);
	let second: Vec<String> = page2["data"]
		.as_array()
		.unwrap()
		.iter()
		.map(|g| g["name"].as_str().unwrap().to_string())
		.collect();
	assert_eq!(second, vec!["Beta Game"]);
	assert_eq!(page2["pagination"]["hasNextPage"], false);
}

#[actix_web::test]
async fn signature_group_games_unknown_group_is_404() {
	let (_pg, db) = start_pg().await;
	let app = test::init_service(build_app!(db)).await;

	let (status, body) = get_json!(
		app,
		"/api/v2/signature-groups/99999999-9999-9999-9999-999999999999/games"
	);
	assert_eq!(status, 404);
	assert_eq!(body["code"], "signature_group_not_found");
}
