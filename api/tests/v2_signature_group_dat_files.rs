//! Postgres-backed tests for the v2 signature-group dat-file listing. Drives the
//! real actix v2 scope so cursor handling, the optional platform filter and the
//! 404 for an unknown signature group are exercised end to end. Requires Docker.

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
const OTHER_SG_ID: &str = "1a1a1a1a-1a1a-1a1a-1a1a-1a1a1a1a1a1a";
const PLATFORM_A: &str = "22222222-2222-2222-2222-222222222222";
const PLATFORM_B: &str = "2b2b2b2b-2b2b-2b2b-2b2b-2b2b2b2b2b2b";

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
        INSERT INTO signature_group (id, name) VALUES
          ('{SG_ID}', 'aaa-sg'),
          ('{OTHER_SG_ID}', 'bbb-sg');
        INSERT INTO platform (id, name) VALUES
          ('{PLATFORM_A}', 'plat-a'),
          ('{PLATFORM_B}', 'plat-b');
        INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id) VALUES
          (gen_random_uuid(), 'dat-a-1', '{PLATFORM_A}', '1.0', '{SG_ID}'),
          (gen_random_uuid(), 'dat-a-2', '{PLATFORM_A}', '1.0', '{SG_ID}'),
          (gen_random_uuid(), 'dat-b-1', '{PLATFORM_B}', '1.0', '{SG_ID}'),
          (gen_random_uuid(), 'dat-other', '{PLATFORM_A}', '1.0', '{OTHER_SG_ID}');
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
async fn lists_only_the_groups_own_dat_files() {
	let (_pg, db) = start_pg().await;
	seed(&db).await;
	let app = test::init_service(build_app!(db)).await;

	let (status, page) = get_json!(app, &format!("/api/v2/signature-groups/{SG_ID}/dat-files"));
	assert_eq!(status, 200);
	let names: Vec<String> = page["data"]
		.as_array()
		.unwrap()
		.iter()
		.map(|d| d["name"].as_str().unwrap().to_string())
		.collect();
	assert_eq!(names, vec!["dat-a-1", "dat-a-2", "dat-b-1"]);
	assert!(names.iter().all(|n| n != "dat-other"));
}

#[actix_web::test]
async fn platform_filter_narrows_the_listing() {
	let (_pg, db) = start_pg().await;
	seed(&db).await;
	let app = test::init_service(build_app!(db)).await;

	let (status, page) = get_json!(
		app,
		&format!("/api/v2/signature-groups/{SG_ID}/dat-files?platformId={PLATFORM_A}")
	);
	assert_eq!(status, 200);
	let names: Vec<String> = page["data"]
		.as_array()
		.unwrap()
		.iter()
		.map(|d| d["name"].as_str().unwrap().to_string())
		.collect();
	assert_eq!(names, vec!["dat-a-1", "dat-a-2"]);
}

#[actix_web::test]
async fn paginates_and_does_not_overlap() {
	let (_pg, db) = start_pg().await;
	seed(&db).await;
	let app = test::init_service(build_app!(db)).await;

	let (_, page) = get_json!(
		app,
		&format!("/api/v2/signature-groups/{SG_ID}/dat-files?limit=2")
	);
	assert_eq!(page["data"].as_array().unwrap().len(), 2);
	assert_eq!(page["pagination"]["hasNextPage"], true);
	let cursor = page["pagination"]["nextCursor"]
		.as_str()
		.unwrap()
		.to_string();

	let (_, page2) = get_json!(
		app,
		&format!("/api/v2/signature-groups/{SG_ID}/dat-files?limit=2&cursor={cursor}")
	);
	let second: Vec<String> = page2["data"]
		.as_array()
		.unwrap()
		.iter()
		.map(|d| d["name"].as_str().unwrap().to_string())
		.collect();
	assert_eq!(second, vec!["dat-b-1"]);
	assert_eq!(page2["pagination"]["hasNextPage"], false);
}

#[actix_web::test]
async fn unknown_signature_group_is_404() {
	let (_pg, db) = start_pg().await;
	let app = test::init_service(build_app!(db)).await;

	let (status, body) = get_json!(
		app,
		"/api/v2/signature-groups/99999999-9999-9999-9999-999999999999/dat-files"
	);
	assert_eq!(status, 404);
	assert_eq!(body["code"], "signature_group_not_found");
}
