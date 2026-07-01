//! Postgres-backed tests for the v2 game sub-resource endpoints: files,
//! mappings, the clone graph and the exact-name lookup. They drive the real
//! actix v2 scope so routing precedence and the public-safe projections are
//! exercised end to end. Require Docker.

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

const PLATFORM_ID: &str = "22222222-2222-2222-2222-222222222222";
const PARENT_GAME_ID: &str = "55555555-5555-5555-5555-555555555555";
const CHILD_GAME_ID: &str = "66666666-6666-6666-6666-666666666666";
const MATCHER_USER_ID: &str = "77777777-7777-7777-7777-777777777777";

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

/// Seed a single platform with a parent game, one clone child and a handful of
/// file records (some current, some retired) plus a public mapping carrying an
/// internally-sourced `manually_matched_by` that must never leak.
async fn seed_graph(db: &DatabaseConnection) {
	let setup = format!(
		r#"
        INSERT INTO signature_group (id, name) VALUES
          ('11111111-1111-1111-1111-111111111111', 'zzz-sg');
        INSERT INTO platform (id, name) VALUES
          ('{PLATFORM_ID}', 'zzz-plat');
        INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id) VALUES
          ('33333333-3333-3333-3333-333333333333', 'zzz-dat',
           '{PLATFORM_ID}', '1.0', '11111111-1111-1111-1111-111111111111');
        INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at) VALUES
          ('44444444-4444-4444-4444-444444444444',
           '33333333-3333-3333-3333-333333333333', 'zzz-import', '1.0',
           'd41d8cd98f00b204e9800998ecf8427e', now());
        INSERT INTO game (id, dat_file_import_id, name, is_current) VALUES
          ('{PARENT_GAME_ID}', '44444444-4444-4444-4444-444444444444', 'Zzz Parent Game', true);
        INSERT INTO game (id, dat_file_import_id, name, is_current, clone_of) VALUES
          ('{CHILD_GAME_ID}', '44444444-4444-4444-4444-444444444444', 'Zzz Clone Game', true, '{PARENT_GAME_ID}');
        INSERT INTO game_file (id, game_id, file_name, is_current) VALUES
          (gen_random_uuid(), '{PARENT_GAME_ID}', 'a-current.rom', true),
          (gen_random_uuid(), '{PARENT_GAME_ID}', 'b-current.rom', true),
          (gen_random_uuid(), '{PARENT_GAME_ID}', 'z-retired.rom', false);
        INSERT INTO "user" (id, username, permissions) VALUES
          ('{MATCHER_USER_ID}', 'zzz-matcher', 'admin');
        INSERT INTO signature_metadata_mapping
          (id, game_id, provider, provider_id, match_type, automatic_match_reason, matched_name, manually_matched_by)
        VALUES
          (gen_random_uuid(), '{PARENT_GAME_ID}', 'igdb', 'igdb-1', 'automatic', 'direct_name', 'Parent Game', '{MATCHER_USER_ID}');
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
			.peer_addr("203.0.113.22:0".parse().unwrap())
			.to_request();
		let resp = test::call_service(&$app, req).await;
		let status = resp.status().as_u16();
		let body = test::read_body(resp).await;
		let json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
		(status, json)
	}};
}

#[actix_web::test]
async fn game_files_default_to_current_only_and_paginate_by_name() {
	let (_pg, db) = start_pg().await;
	seed_graph(&db).await;
	let app = test::init_service(build_app!(db)).await;

	let (status, page) = get_json!(app, &format!("/api/v2/games/{PARENT_GAME_ID}/files"));
	assert_eq!(status, 200);
	let names: Vec<String> = page["data"]
		.as_array()
		.unwrap()
		.iter()
		.map(|f| f["fileName"].as_str().unwrap().to_string())
		.collect();
	assert_eq!(names, vec!["a-current.rom", "b-current.rom"]);

	let (_, all) = get_json!(
		app,
		&format!("/api/v2/games/{PARENT_GAME_ID}/files?currentOnly=false")
	);
	assert_eq!(all["data"].as_array().unwrap().len(), 3);
}

#[actix_web::test]
async fn game_files_unknown_game_is_404() {
	let (_pg, db) = start_pg().await;
	let app = test::init_service(build_app!(db)).await;

	let (status, body) = get_json!(
		app,
		"/api/v2/games/00000000-0000-0000-0000-000000000000/files"
	);
	assert_eq!(status, 404);
	assert_eq!(body["code"], "game_not_found");
}

#[actix_web::test]
async fn game_mappings_omit_internal_fields_and_filter() {
	let (_pg, db) = start_pg().await;
	seed_graph(&db).await;
	let app = test::init_service(build_app!(db)).await;

	let (status, body) = get_json!(app, &format!("/api/v2/games/{PARENT_GAME_ID}/mappings"));
	assert_eq!(status, 200);
	let rows = body.as_array().unwrap();
	assert_eq!(rows.len(), 1);
	let mapping = &rows[0];
	assert_eq!(mapping["provider"], "IGDB");
	assert_eq!(mapping["matchedName"], "Parent Game");
	assert!(mapping.get("manually_matched_by").is_none());
	assert!(mapping.get("cross_match_last_tried_at").is_none());

	let (_, filtered) = get_json!(
		app,
		&format!("/api/v2/games/{PARENT_GAME_ID}/mappings?matchType=Manual")
	);
	assert_eq!(filtered.as_array().unwrap().len(), 0);
}

#[actix_web::test]
async fn game_clones_resolves_children_parent_and_siblings() {
	let (_pg, db) = start_pg().await;
	seed_graph(&db).await;
	let app = test::init_service(build_app!(db)).await;

	let (status, children) = get_json!(
		app,
		&format!("/api/v2/games/{PARENT_GAME_ID}/clones?direction=children")
	);
	assert_eq!(status, 200);
	assert_eq!(children["children"].as_array().unwrap().len(), 1);
	assert_eq!(children["children"][0]["id"], CHILD_GAME_ID);
	assert_eq!(
		children["cloneGraphComplete"], true,
		"a fully resolved dat file reports a complete clone graph"
	);

	let (_, parent) = get_json!(
		app,
		&format!("/api/v2/games/{CHILD_GAME_ID}/clones?direction=parent")
	);
	assert_eq!(parent["parent"]["id"], PARENT_GAME_ID);

	let (_, siblings) = get_json!(
		app,
		&format!("/api/v2/games/{CHILD_GAME_ID}/clones?direction=siblings")
	);
	assert!(
		siblings["children"]
			.as_array()
			.map(|nodes| nodes.iter().all(|n| n["id"] != CHILD_GAME_ID))
			.unwrap_or(true),
		"a game is never its own sibling"
	);
}

#[actix_web::test]
async fn clone_graph_reports_incomplete_while_resolution_lags() {
	let (_pg, db) = start_pg().await;
	// One game in a dat file knows its signature-group-internal clone-of id but
	// the deferred pass has not linked `clone_of` yet, so the graph for that dat
	// file must report itself incomplete.
	let lagging_game = "55555555-5555-5555-5555-555555555555";
	let setup = format!(
		r#"
        INSERT INTO signature_group (id, name) VALUES
          ('11111111-1111-1111-1111-111111111111', 'zzz-sg');
        INSERT INTO platform (id, name) VALUES
          ('{PLATFORM_ID}', 'zzz-plat');
        INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id) VALUES
          ('33333333-3333-3333-3333-333333333333', 'zzz-dat',
           '{PLATFORM_ID}', '1.0', '11111111-1111-1111-1111-111111111111');
        INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at) VALUES
          ('44444444-4444-4444-4444-444444444444',
           '33333333-3333-3333-3333-333333333333', 'zzz-import', '1.0',
           'd41d8cd98f00b204e9800998ecf8427e', now());
        INSERT INTO game (id, dat_file_import_id, name, is_current, signature_group_internal_clone_of_id) VALUES
          ('{lagging_game}', '44444444-4444-4444-4444-444444444444', 'Zzz Lagging Clone', true, 7);
    "#
	);
	db.execute_unprepared(&setup).await.unwrap();
	let app = test::init_service(build_app!(db)).await;

	let (status, graph) = get_json!(app, &format!("/api/v2/games/{lagging_game}/clones"));
	assert_eq!(status, 200);
	assert_eq!(
		graph["cloneGraphComplete"], false,
		"an unresolved signature-group-internal clone in the dat file marks the graph incomplete"
	);
}

#[actix_web::test]
async fn game_file_by_id_returns_the_file_and_404s_when_unknown() {
	let (_pg, db) = start_pg().await;
	seed_graph(&db).await;
	let app = test::init_service(build_app!(db)).await;

	let (_, files) = get_json!(
		app,
		&format!("/api/v2/games/{PARENT_GAME_ID}/files?currentOnly=false")
	);
	let file_id = files["data"]
		.as_array()
		.unwrap()
		.iter()
		.find(|f| f["fileName"] == "a-current.rom")
		.unwrap()["id"]
		.as_str()
		.unwrap()
		.to_string();

	let (status, body) = get_json!(app, &format!("/api/v2/game-files/{file_id}"));
	assert_eq!(status, 200);
	assert_eq!(body["id"], file_id);
	assert_eq!(body["gameId"], PARENT_GAME_ID);
	assert_eq!(body["fileName"], "a-current.rom");

	let (status, body) = get_json!(
		app,
		"/api/v2/game-files/00000000-0000-0000-0000-000000000000"
	);
	assert_eq!(status, 404);
	assert_eq!(body["code"], "game_file_not_found");
}

#[actix_web::test]
async fn game_by_name_is_case_insensitive_and_platform_scoped() {
	let (_pg, db) = start_pg().await;
	seed_graph(&db).await;
	let app = test::init_service(build_app!(db)).await;

	let (status, hits) = get_json!(
		app,
		&format!("/api/v2/games/by-name?name=zzz%20parent%20game&platformId={PLATFORM_ID}")
	);
	assert_eq!(status, 200);
	let rows = hits.as_array().unwrap();
	assert_eq!(rows.len(), 1);
	assert_eq!(rows[0]["id"], PARENT_GAME_ID);

	let (_, miss) = get_json!(
		app,
		"/api/v2/games/by-name?name=nope&platformId=00000000-0000-0000-0000-000000000000"
	);
	assert_eq!(miss.as_array().unwrap().len(), 0);
}
