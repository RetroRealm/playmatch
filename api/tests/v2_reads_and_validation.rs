//! Postgres-backed tests for v2 single-resource reads and input validation:
//! company/platform get-by-id (found and 404), core game reads, and the
//! empty/too-long search query guards. They drive the real actix v2 scope so
//! routing and the v2 error envelope are exercised end to end. Require Docker.

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
const COMPANY_ID: &str = "22222222-2222-2222-2222-222222222222";
const PLATFORM_ID: &str = "33333333-3333-3333-3333-333333333333";
const DAT_ID: &str = "44444444-4444-4444-4444-444444444444";
const IMPORT_ID: &str = "55555555-5555-5555-5555-555555555555";
const GAME_ID: &str = "66666666-6666-6666-6666-666666666666";
const MISSING: &str = "00000000-0000-0000-0000-0000000000ff";

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

async fn seed(db: &DatabaseConnection) {
	let sql = format!(
		r#"
        INSERT INTO signature_group (id, name) VALUES ('{SG_ID}', 'zzz-sg');
        INSERT INTO company (id, name) VALUES ('{COMPANY_ID}', 'zzz-company');
        INSERT INTO platform (id, name, company_id) VALUES ('{PLATFORM_ID}', 'zzz-platform', '{COMPANY_ID}');
        INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id) VALUES
          ('{DAT_ID}', 'zzz-dat', '{PLATFORM_ID}', '1.0', '{SG_ID}');
        INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at) VALUES
          ('{IMPORT_ID}', '{DAT_ID}', 'zzz-import', '1.0', 'd41d8cd98f00b204e9800998ecf8427e', now());
        INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id) VALUES
          ('{GAME_ID}', '{IMPORT_ID}', 'zzz-game', true, '{IMPORT_ID}');
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
			.peer_addr("203.0.113.77:0".parse().unwrap())
			.to_request();
		let resp = test::call_service(&$app, req).await;
		let status = resp.status().as_u16();
		let body = test::read_body(resp).await;
		let json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
		(status, json)
	}};
}

#[actix_web::test]
async fn company_by_id_found_and_404() {
	let (_pg, db) = start_pg().await;
	seed(&db).await;
	let app = test::init_service(build_app!(db)).await;

	let (status, body) = get_json!(app, &format!("/api/v2/companies/{COMPANY_ID}"));
	assert_eq!(status, 200);
	assert_eq!(body["id"], COMPANY_ID);
	assert_eq!(body["name"], "zzz-company");

	let (status, body) = get_json!(app, &format!("/api/v2/companies/{MISSING}"));
	assert_eq!(status, 404);
	assert_eq!(body["code"], "company_not_found");
}

#[actix_web::test]
async fn platform_by_id_found_and_404() {
	let (_pg, db) = start_pg().await;
	seed(&db).await;
	let app = test::init_service(build_app!(db)).await;

	let (status, body) = get_json!(app, &format!("/api/v2/platforms/{PLATFORM_ID}"));
	assert_eq!(status, 200);
	assert_eq!(body["id"], PLATFORM_ID);
	assert_eq!(body["name"], "zzz-platform");

	let (status, body) = get_json!(app, &format!("/api/v2/platforms/{MISSING}"));
	assert_eq!(status, 404);
	assert_eq!(body["code"], "platform_not_found");
}

#[actix_web::test]
async fn game_by_id_found_and_404() {
	let (_pg, db) = start_pg().await;
	seed(&db).await;
	let app = test::init_service(build_app!(db)).await;

	let (status, body) = get_json!(app, &format!("/api/v2/games/{GAME_ID}"));
	assert_eq!(status, 200);
	assert_eq!(body["id"], GAME_ID);
	assert_eq!(body["name"], "zzz-game");

	let (status, body) = get_json!(app, &format!("/api/v2/games/{MISSING}"));
	assert_eq!(status, 404);
	assert_eq!(body["code"], "game_not_found");
}

#[actix_web::test]
async fn game_with_relations_404s_on_unknown_id() {
	let (_pg, db) = start_pg().await;
	let app = test::init_service(build_app!(db)).await;

	let (status, body) = get_json!(app, &format!("/api/v2/games/{MISSING}/with-relations"));
	assert_eq!(status, 404);
	assert_eq!(body["code"], "game_not_found");
}

#[actix_web::test]
async fn company_search_rejects_empty_query() {
	let (_pg, db) = start_pg().await;
	let app = test::init_service(build_app!(db)).await;

	let (status, body) = get_json!(app, "/api/v2/companies/search?query=");
	assert_eq!(status, 400);
	assert_eq!(body["code"], "empty_query");
}

#[actix_web::test]
async fn game_search_rejects_a_query_over_the_length_cap() {
	let (_pg, db) = start_pg().await;
	let app = test::init_service(build_app!(db)).await;

	let too_long = "a".repeat(201);
	let (status, body) = get_json!(app, &format!("/api/v2/games/search?query={too_long}"));
	assert_eq!(status, 400);
	assert_eq!(body["code"], "query_too_long");
}
