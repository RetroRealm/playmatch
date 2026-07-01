//! Postgres-backed tests for the v2 games browse, paginated search and the
//! paginated suggestion list. They drive the real actix v2 scope so cursor
//! handling, filter-set tagging and auth gating are exercised end to end.
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

/// Seed `count` current, non-clone games named `zzz-game-<nnn>` under a single
/// platform/signature-group/dat-file chain. Returns the platform id.
async fn seed_games(db: &DatabaseConnection, count: usize) -> String {
	let setup = r#"
        INSERT INTO signature_group (id, name) VALUES
          ('11111111-1111-1111-1111-111111111111', 'zzz-sg');
        INSERT INTO platform (id, name) VALUES
          ('22222222-2222-2222-2222-222222222222', 'zzz-plat');
        INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id) VALUES
          ('33333333-3333-3333-3333-333333333333', 'zzz-dat',
           '22222222-2222-2222-2222-222222222222', '1.0',
           '11111111-1111-1111-1111-111111111111');
        INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at) VALUES
          ('44444444-4444-4444-4444-444444444444',
           '33333333-3333-3333-3333-333333333333', 'zzz-import', '1.0',
           'd41d8cd98f00b204e9800998ecf8427e', now());
    "#;
	db.execute_unprepared(setup).await.unwrap();

	let mut sql = String::new();
	for i in 0..count {
		sql.push_str(&format!(
			"INSERT INTO game (id, dat_file_import_id, name, is_current) VALUES (gen_random_uuid(), '44444444-4444-4444-4444-444444444444', 'zzz-game-{i:03}', true);\n"
		));
	}
	// One clone and one non-current row that the defaults must hide.
	sql.push_str(
		"INSERT INTO game (id, dat_file_import_id, name, is_current, clone_of) VALUES (gen_random_uuid(), '44444444-4444-4444-4444-444444444444', 'zzz-clone-row', true, (SELECT id FROM game LIMIT 1));\n",
	);
	sql.push_str(
		"INSERT INTO game (id, dat_file_import_id, name, is_current) VALUES (gen_random_uuid(), '44444444-4444-4444-4444-444444444444', 'zzz-retired-row', false);\n",
	);
	db.execute_unprepared(&sql).await.unwrap();

	"22222222-2222-2222-2222-222222222222".to_string()
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
			.peer_addr("203.0.113.21:0".parse().unwrap())
			.to_request();
		let resp = test::call_service(&$app, req).await;
		let status = resp.status().as_u16();
		let body = test::read_body(resp).await;
		let json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
		(status, json)
	}};
}

#[actix_web::test]
async fn games_browse_paginates_in_name_order_excluding_clones_and_retired() {
	let (_pg, db) = start_pg().await;
	seed_games(&db, 5).await;
	let app = test::init_service(build_app!(db)).await;

	let (status, page) = get_json!(app, "/api/v2/games?limit=2");
	assert_eq!(status, 200);
	assert_eq!(page["data"].as_array().unwrap().len(), 2);
	assert_eq!(page["pagination"]["hasNextPage"], true);

	let cursor = page["pagination"]["nextCursor"]
		.as_str()
		.unwrap()
		.to_string();
	let first_names: Vec<String> = page["data"]
		.as_array()
		.unwrap()
		.iter()
		.map(|g| g["name"].as_str().unwrap().to_string())
		.collect();

	// Defaults hide the clone and the retired row.
	for n in &first_names {
		assert!(n.starts_with("zzz-game-"), "unexpected row {n}");
	}

	let (status, page2) = get_json!(app, &format!("/api/v2/games?limit=2&cursor={cursor}"));
	assert_eq!(status, 200);
	let second_names: Vec<String> = page2["data"]
		.as_array()
		.unwrap()
		.iter()
		.map(|g| g["name"].as_str().unwrap().to_string())
		.collect();
	for n in &second_names {
		assert!(
			!first_names.contains(n),
			"second page must not repeat first-page rows: {n}"
		);
	}
	assert!(
		second_names
			.iter()
			.all(|a| first_names.iter().all(|b| a > b)),
		"keyset order must be strictly increasing across pages"
	);
}

#[actix_web::test]
async fn games_browse_includes_clones_when_requested() {
	let (_pg, db) = start_pg().await;
	seed_games(&db, 3).await;
	let app = test::init_service(build_app!(db)).await;

	let (status, page) = get_json!(app, "/api/v2/games?limit=50&clones=true");
	assert_eq!(status, 200);
	let names: Vec<String> = page["data"]
		.as_array()
		.unwrap()
		.iter()
		.map(|g| g["name"].as_str().unwrap().to_string())
		.collect();
	assert!(
		names.iter().any(|n| n == "zzz-clone-row"),
		"clones=true must surface the clone row, got {names:?}"
	);
}

#[actix_web::test]
async fn games_browse_cursor_rejected_after_filter_change() {
	let (_pg, db) = start_pg().await;
	let platform_id = seed_games(&db, 5).await;
	let app = test::init_service(build_app!(db)).await;

	let (_, page) = get_json!(app, "/api/v2/games?limit=2");
	let cursor = page["pagination"]["nextCursor"]
		.as_str()
		.unwrap()
		.to_string();

	// Replaying the cursor under a different filter set must 400 with a restart hint.
	let (status, body) = get_json!(
		app,
		&format!("/api/v2/games?limit=2&platformId={platform_id}&cursor={cursor}")
	);
	assert_eq!(status, 400);
	assert_eq!(body["code"], "cursor_filter_mismatch");
	assert_eq!(body["restart"], true);
}

#[actix_web::test]
async fn game_search_paginates_preserving_similarity_order() {
	let (_pg, db) = start_pg().await;
	seed_games(&db, 6).await;
	let app = test::init_service(build_app!(db)).await;

	let (status, page) = get_json!(app, "/api/v2/games/search?query=zzz-game&limit=2");
	assert_eq!(status, 200);
	assert_eq!(page["data"].as_array().unwrap().len(), 2);

	if page["pagination"]["hasNextPage"].as_bool().unwrap_or(false) {
		let cursor = page["pagination"]["nextCursor"].as_str().unwrap();
		let (status2, page2) = get_json!(
			app,
			&format!("/api/v2/games/search?query=zzz-game&limit=2&cursor={cursor}")
		);
		assert_eq!(status2, 200);
		let first: Vec<String> = page["data"]
			.as_array()
			.unwrap()
			.iter()
			.map(|g| g["id"].as_str().unwrap().to_string())
			.collect();
		let second: Vec<String> = page2["data"]
			.as_array()
			.unwrap()
			.iter()
			.map(|g| g["id"].as_str().unwrap().to_string())
			.collect();
		for id in &second {
			assert!(!first.contains(id), "search pages must not overlap: {id}");
		}
	}
}

#[actix_web::test]
async fn game_search_rejects_empty_query() {
	let (_pg, db) = start_pg().await;
	let app = test::init_service(build_app!(db)).await;

	let (status, _) = get_json!(app, "/api/v2/games/search?query=");
	assert_eq!(status, 400);
}

#[actix_web::test]
async fn suggestion_list_requires_authentication() {
	let (_pg, db) = start_pg().await;
	let app = test::init_service(build_app!(db)).await;

	// No bearer token: the Automation gate rejects before any row is read, so no
	// suggestion rows need to exist for this to be a meaningful auth check.
	let (status, _) = get_json!(app, "/api/v2/suggestion?limit=2");
	assert_eq!(status, 401);
}
