//! Postgres-backed tests for the v2 reverse lookups: hash to dat files / signature
//! groups and game to dat files. They drive the real actix v2 scope so the hash
//! cascade, presence aggregation and group collapse are exercised end to end.
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
const PLATFORM_ID: &str = "22222222-2222-2222-2222-222222222222";
const DAT_ID: &str = "33333333-3333-3333-3333-333333333333";
const IMPORT_OLD: &str = "44444444-4444-4444-4444-444444444444";
const IMPORT_NEW: &str = "55555555-5555-5555-5555-555555555555";
const GAME_ID: &str = "66666666-6666-6666-6666-666666666666";
const GAME_FILE_ID: &str = "77777777-7777-7777-7777-777777777777";

const SHA1: &str = "da39a3ee5e6b4b0d3255bfef95601890afd80709";
const SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
const CRC: &str = "00000000";

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

/// One dat file with two imports (the newer one is the current release), a single
/// game with a hashed game file, and a presence row for the file in each import.
async fn seed(db: &DatabaseConnection) {
	let setup = format!(
		r#"
        INSERT INTO signature_group (id, name) VALUES ('{SG_ID}', 'No-Intro');
        INSERT INTO platform (id, name) VALUES ('{PLATFORM_ID}', 'NES');
        INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id, latest_dat_file_import_id)
          VALUES ('{DAT_ID}', 'nes-dat', '{PLATFORM_ID}', '2.0', '{SG_ID}', '{IMPORT_NEW}');
        INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at) VALUES
          ('{IMPORT_OLD}', '{DAT_ID}', 'import-old', '1.0', 'd41d8cd98f00b204e9800998ecf8427e', now() - interval '2 days'),
          ('{IMPORT_NEW}', '{DAT_ID}', 'import-new', '2.0', 'd41d8cd98f00b204e9800998ecf8427e', now());
        INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id)
          VALUES ('{GAME_ID}', '{IMPORT_NEW}', 'Some Game', true, '{IMPORT_NEW}');
        INSERT INTO game_file (id, game_id, file_name, sha1, is_current, last_seen_dat_file_import_id)
          VALUES ('{GAME_FILE_ID}', '{GAME_ID}', 'some-game.nes', '{SHA1}', true, '{IMPORT_NEW}');
        INSERT INTO game_file_presence (id, game_file_id, dat_file_import_id) VALUES
          (gen_random_uuid(), '{GAME_FILE_ID}', '{IMPORT_OLD}'),
          (gen_random_uuid(), '{GAME_FILE_ID}', '{IMPORT_NEW}');
    "#
	);
	db.execute_unprepared(&setup).await.unwrap();
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
async fn by_hash_returns_dat_with_first_last_seen_and_current_flag() {
	let (_pg, db) = start_pg().await;
	seed(&db).await;
	let app = test::init_service(build_app!(db)).await;

	let (status, body) = get_json!(app, &format!("/api/v2/dat-files/by-hash?sha1={SHA1}"));
	assert_eq!(status, 200);
	let entries = body.as_array().unwrap();
	assert_eq!(entries.len(), 1);
	let dat = &entries[0];
	assert_eq!(dat["id"], DAT_ID);
	assert_eq!(dat["name"], "nes-dat");
	assert_eq!(dat["signatureGroup"]["name"], "No-Intro");
	assert_eq!(dat["firstSeenImport"]["version"], "1.0");
	assert_eq!(dat["lastSeenImport"]["version"], "2.0");
	assert_eq!(dat["isCurrentInLatest"], true);
}

#[actix_web::test]
async fn by_hash_level_group_collapses_to_signature_group() {
	let (_pg, db) = start_pg().await;
	seed(&db).await;
	let app = test::init_service(build_app!(db)).await;

	let (status, body) = get_json!(
		app,
		&format!("/api/v2/dat-files/by-hash?sha1={SHA1}&level=group")
	);
	assert_eq!(status, 200);
	let entries = body.as_array().unwrap();
	assert_eq!(entries.len(), 1);
	let group = &entries[0];
	assert_eq!(group["signatureGroup"]["name"], "No-Intro");
	assert!(group["dat_file"].is_null());
	assert_eq!(group["isCurrentInLatest"], true);
}

#[actix_web::test]
async fn by_hash_falls_through_to_a_weaker_supplied_hash() {
	let (_pg, db) = start_pg().await;
	seed(&db).await;
	let app = test::init_service(build_app!(db)).await;

	// sha256 is supplied but matches nothing; the cascade falls through to the
	// sha1 that does match, exactly as the identify cascade resolves a file.
	let (status, body) = get_json!(
		app,
		&format!("/api/v2/dat-files/by-hash?sha256={SHA256}&sha1={SHA1}")
	);
	assert_eq!(status, 200);
	assert_eq!(body.as_array().unwrap()[0]["id"], DAT_ID);
}

#[actix_web::test]
async fn by_hash_unmatched_is_404_and_no_hash_is_400() {
	let (_pg, db) = start_pg().await;
	seed(&db).await;
	let app = test::init_service(build_app!(db)).await;

	let (missing, _) = get_json!(app, &format!("/api/v2/dat-files/by-hash?crc={CRC}"));
	assert_eq!(missing, 404);

	let (no_hash, no_hash_body) = get_json!(app, "/api/v2/dat-files/by-hash");
	assert_eq!(no_hash, 400);
	assert_eq!(no_hash_body["code"], "invalid_hash");

	let (malformed, malformed_body) = get_json!(app, "/api/v2/dat-files/by-hash?sha1=nothex");
	assert_eq!(malformed, 400);
	assert_eq!(malformed_body["code"], "invalid_hash");
}

#[actix_web::test]
async fn game_dat_files_mirrors_the_hash_lookup() {
	let (_pg, db) = start_pg().await;
	seed(&db).await;
	let app = test::init_service(build_app!(db)).await;

	let (status, body) = get_json!(app, &format!("/api/v2/games/{GAME_ID}/dat-files"));
	assert_eq!(status, 200);
	let entries = body.as_array().unwrap();
	assert_eq!(entries.len(), 1);
	assert_eq!(entries[0]["id"], DAT_ID);
	assert_eq!(entries[0]["isCurrentInLatest"], true);

	let (group_status, group_body) = get_json!(
		app,
		&format!("/api/v2/games/{GAME_ID}/dat-files?level=group")
	);
	assert_eq!(group_status, 200);
	assert_eq!(
		group_body.as_array().unwrap()[0]["signatureGroup"]["name"],
		"No-Intro"
	);

	let (missing, _) = get_json!(
		app,
		"/api/v2/games/99999999-9999-9999-9999-999999999999/dat-files"
	);
	assert_eq!(missing, 404);
}
