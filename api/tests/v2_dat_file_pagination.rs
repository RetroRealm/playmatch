//! Postgres-backed tests for the v2 DAT catalogue endpoints: the paginated and
//! filtered dat-file list, the single dat-file read with its game counts, and
//! the paginated games-in-a-dat listing including hydration. They drive the real
//! actix v2 scope so cursor handling and filter-set tagging are exercised end to
//! end. Require Docker.

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
const IMPORT_ID: &str = "44444444-4444-4444-4444-444444444444";

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

/// Seed a single signature group, platform, dat file and import, then `games`
/// current games plus one retired (non-current) game so the games listing can
/// exercise the current_only default.
async fn seed_catalogue(db: &DatabaseConnection, games: usize) {
	let setup = format!(
		r#"
        INSERT INTO signature_group (id, name) VALUES ('{SG_ID}', 'zzz-sg');
        INSERT INTO platform (id, name) VALUES ('{PLATFORM_ID}', 'zzz-plat');
        INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id, tags, subset) VALUES
          ('{DAT_ID}', 'zzz-dat', '{PLATFORM_ID}', '1.0', '{SG_ID}', ARRAY['redump']::text[], 'demos');
        INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at) VALUES
          ('{IMPORT_ID}', '{DAT_ID}', 'zzz-import', '1.0', 'd41d8cd98f00b204e9800998ecf8427e', now());
    "#
	);
	db.execute_unprepared(&setup).await.unwrap();

	let mut sql = String::new();
	for i in 0..games {
		sql.push_str(&format!(
			"INSERT INTO game (id, dat_file_import_id, name, is_current) VALUES (gen_random_uuid(), '{IMPORT_ID}', 'zzz-game-{i:03}', true);\n"
		));
	}
	sql.push_str(&format!(
		"INSERT INTO game (id, dat_file_import_id, name, is_current) VALUES (gen_random_uuid(), '{IMPORT_ID}', 'zzz-retired', false);\n"
	));
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
async fn dat_files_list_returns_summary_with_related_names() {
	let (_pg, db) = start_pg().await;
	seed_catalogue(&db, 2).await;
	let app = test::init_service(build_app!(db)).await;

	let (status, page) = get_json!(app, "/api/v2/dat-files?limit=10");
	assert_eq!(status, 200);
	let data = page["data"].as_array().unwrap();
	assert_eq!(data.len(), 1);
	let dat = &data[0];
	assert_eq!(dat["name"], "zzz-dat");
	assert_eq!(dat["signatureGroup"]["name"], "zzz-sg");
	assert_eq!(dat["platform"]["name"], "zzz-plat");
	assert_eq!(dat["subset"], "demos");
	assert_eq!(dat["tags"][0], "redump");
	assert_eq!(dat["latestDatFileImport"]["version"], "1.0");
}

#[actix_web::test]
async fn dat_files_filter_by_tag_and_name() {
	let (_pg, db) = start_pg().await;
	seed_catalogue(&db, 1).await;
	let app = test::init_service(build_app!(db)).await;

	let (_, hit) = get_json!(app, "/api/v2/dat-files?tag=redump&name=zzz");
	assert_eq!(hit["data"].as_array().unwrap().len(), 1);

	let (_, miss) = get_json!(app, "/api/v2/dat-files?tag=nointro");
	assert_eq!(miss["data"].as_array().unwrap().len(), 0);
	assert_eq!(miss["pagination"]["hasNextPage"], false);
	assert!(miss["pagination"]["nextCursor"].is_null());
}

#[actix_web::test]
async fn dat_file_detail_reports_game_counts() {
	let (_pg, db) = start_pg().await;
	seed_catalogue(&db, 3).await;
	let app = test::init_service(build_app!(db)).await;

	let (status, dat) = get_json!(app, &format!("/api/v2/dat-files/{DAT_ID}"));
	assert_eq!(status, 200);
	assert_eq!(dat["gameCount"], 4);
	assert_eq!(dat["currentGameCount"], 3);

	let (missing, _) = get_json!(
		app,
		"/api/v2/dat-files/55555555-5555-5555-5555-555555555555"
	);
	assert_eq!(missing, 404);
}

#[actix_web::test]
async fn dat_file_games_default_hides_retired_and_paginates() {
	let (_pg, db) = start_pg().await;
	seed_catalogue(&db, 5).await;
	let app = test::init_service(build_app!(db)).await;

	let (status, page) = get_json!(app, &format!("/api/v2/dat-files/{DAT_ID}/games?limit=2"));
	assert_eq!(status, 200);
	assert_eq!(page["data"].as_array().unwrap().len(), 2);
	assert_eq!(page["pagination"]["hasNextPage"], true);
	let cursor = page["pagination"]["nextCursor"]
		.as_str()
		.unwrap()
		.to_string();

	let first: Vec<String> = page["data"]
		.as_array()
		.unwrap()
		.iter()
		.map(|g| g["name"].as_str().unwrap().to_string())
		.collect();
	assert!(first.iter().all(|n| n != "zzz-retired"));

	let (_, page2) = get_json!(
		app,
		&format!("/api/v2/dat-files/{DAT_ID}/games?limit=2&cursor={cursor}")
	);
	let second: Vec<String> = page2["data"]
		.as_array()
		.unwrap()
		.iter()
		.map(|g| g["name"].as_str().unwrap().to_string())
		.collect();
	for n in &second {
		assert!(!first.contains(n), "pages must not overlap: {n}");
	}
}

#[actix_web::test]
async fn dat_file_games_current_only_false_includes_retired() {
	let (_pg, db) = start_pg().await;
	seed_catalogue(&db, 1).await;
	let app = test::init_service(build_app!(db)).await;

	let (status, page) = get_json!(
		app,
		&format!("/api/v2/dat-files/{DAT_ID}/games?currentOnly=false&limit=50")
	);
	assert_eq!(status, 200);
	let names: Vec<String> = page["data"]
		.as_array()
		.unwrap()
		.iter()
		.map(|g| g["name"].as_str().unwrap().to_string())
		.collect();
	assert!(names.iter().any(|n| n == "zzz-retired"));
}

#[actix_web::test]
async fn dat_file_games_for_unknown_dat_is_404() {
	let (_pg, db) = start_pg().await;
	let app = test::init_service(build_app!(db)).await;

	let (status, body) = get_json!(
		app,
		"/api/v2/dat-files/55555555-5555-5555-5555-555555555555/games"
	);
	assert_eq!(status, 404);
	assert_eq!(body["code"], "dat_file_not_found");
}

/// Seed a single dat file with `count` imports spread over distinct timestamps so
/// the newest-first timeline has a deterministic order. Returns the import ids in
/// chronological (oldest first) order.
async fn seed_imports(db: &DatabaseConnection, count: usize) -> Vec<String> {
	let base = format!(
		r#"
        INSERT INTO signature_group (id, name) VALUES ('{SG_ID}', 'zzz-sg');
        INSERT INTO platform (id, name) VALUES ('{PLATFORM_ID}', 'zzz-plat');
        INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id) VALUES
          ('{DAT_ID}', 'zzz-dat', '{PLATFORM_ID}', '1.0', '{SG_ID}');
    "#
	);
	db.execute_unprepared(&base).await.unwrap();

	let mut ids = Vec::new();
	let mut sql = String::new();
	for i in 0..count {
		let id = format!("44444444-4444-4444-4444-4444444444{i:02}");
		ids.push(id.clone());
		sql.push_str(&format!(
			"INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at) VALUES \
             ('{id}', '{DAT_ID}', 'zzz-import-{i:02}', '1.{i}', 'd41d8cd98f00b204e9800998ecf8427e', now() + interval '{i} hour');\n"
		));
	}
	db.execute_unprepared(&sql).await.unwrap();
	ids
}

#[actix_web::test]
async fn dat_file_imports_are_newest_first_and_paginate() {
	let (_pg, db) = start_pg().await;
	let ids = seed_imports(&db, 4).await;
	let app = test::init_service(build_app!(db)).await;

	let (status, page) = get_json!(app, &format!("/api/v2/dat-files/{DAT_ID}/imports?limit=2"));
	assert_eq!(status, 200);
	let data = page["data"].as_array().unwrap();
	assert_eq!(data.len(), 2);
	// Newest seeded import wins; md5 must not leak into the timeline projection.
	assert_eq!(data[0]["id"], *ids.last().unwrap());
	assert!(data[0]["md5"].is_null());
	assert_eq!(data[0]["name"], "zzz-import-03");
	assert_eq!(page["pagination"]["hasNextPage"], true);

	let cursor = page["pagination"]["nextCursor"]
		.as_str()
		.unwrap()
		.to_string();
	let (_, page2) = get_json!(
		app,
		&format!("/api/v2/dat-files/{DAT_ID}/imports?limit=2&cursor={cursor}")
	);
	let second: Vec<String> = page2["data"]
		.as_array()
		.unwrap()
		.iter()
		.map(|i| i["id"].as_str().unwrap().to_string())
		.collect();
	let first: Vec<String> = data
		.iter()
		.map(|i| i["id"].as_str().unwrap().to_string())
		.collect();
	for id in &second {
		assert!(!first.contains(id), "pages must not overlap: {id}");
	}
}

#[actix_web::test]
async fn dat_file_imports_for_unknown_dat_is_404() {
	let (_pg, db) = start_pg().await;
	let app = test::init_service(build_app!(db)).await;

	let (status, body) = get_json!(
		app,
		"/api/v2/dat-files/55555555-5555-5555-5555-555555555555/imports"
	);
	assert_eq!(status, 404);
	assert_eq!(body["code"], "dat_file_not_found");
}

#[actix_web::test]
async fn single_dat_file_import_is_addressable_and_scoped() {
	let (_pg, db) = start_pg().await;
	let ids = seed_imports(&db, 2).await;
	let app = test::init_service(build_app!(db)).await;

	let import_id = &ids[0];
	let (status, body) = get_json!(
		app,
		&format!("/api/v2/dat-files/{DAT_ID}/imports/{import_id}")
	);
	assert_eq!(status, 200);
	assert_eq!(body["id"], *import_id);
	assert!(body["md5"].is_null());

	// An import that exists but belongs to another (here, nonexistent) dat file
	// reads as not found rather than leaking across the boundary.
	let (mismatch, _) = get_json!(
		app,
		&format!("/api/v2/dat-files/55555555-5555-5555-5555-555555555555/imports/{import_id}")
	);
	assert_eq!(mismatch, 404);

	let (missing_import, _) = get_json!(
		app,
		&format!("/api/v2/dat-files/{DAT_ID}/imports/44444444-4444-4444-4444-4444444444ff")
	);
	assert_eq!(missing_import, 404);
}

#[actix_web::test]
async fn dat_file_cursor_from_another_filter_set_is_rejected() {
	let (_pg, db) = start_pg().await;
	seed_catalogue(&db, 4).await;
	let app = test::init_service(build_app!(db)).await;

	let (_, page) = get_json!(
		app,
		"/api/v2/dat-files/33333333-3333-3333-3333-333333333333/games?limit=2"
	);
	let cursor = page["pagination"]["nextCursor"]
		.as_str()
		.unwrap()
		.to_string();

	// Replaying a current_only=true cursor against current_only=false must 400
	// because the filter tags differ.
	let (status, body) = get_json!(
		app,
		&format!("/api/v2/dat-files/{DAT_ID}/games?currentOnly=false&cursor={cursor}")
	);
	assert_eq!(status, 400);
	assert_eq!(body["code"], "cursor_filter_mismatch");
	assert_eq!(body["restart"], true);
}
