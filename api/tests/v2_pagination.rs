//! Postgres-backed tests for the v2 keyset-paginated reference lists. They drive
//! the real actix v2 scope so cursor handling, the envelope shape, and
//! with_total honoring are all exercised end to end. Require Docker.

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

/// Seed `count` signature groups named `zzz-pg-<nnn>` so they sort after every
/// row the migrations insert and form a deterministic ordered tail.
async fn seed_signature_groups(db: &DatabaseConnection, count: usize) {
	let mut sql = String::new();
	for i in 0..count {
		sql.push_str(&format!(
			"INSERT INTO signature_group (id, name) VALUES (gen_random_uuid(), 'zzz-pg-{i:03}');\n"
		));
	}
	db.execute_unprepared(&sql).await.unwrap();
}

async fn seed_platforms(db: &DatabaseConnection, count: usize) {
	let mut sql = String::new();
	for i in 0..count {
		sql.push_str(&format!(
			"INSERT INTO platform (id, name) VALUES (gen_random_uuid(), 'zzz-plat-{i:03}');\n"
		));
	}
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
async fn signature_groups_paginate_and_cursor_walks_the_ordered_set() {
	let (_pg, db) = start_pg().await;
	seed_signature_groups(&db, 5).await;
	let app = test::init_service(build_app!(db)).await;

	let (status, page) = get_json!(app, "/api/v2/signature-groups?limit=2");
	assert_eq!(status, 200);
	assert_eq!(page["data"].as_array().unwrap().len(), 2);
	assert_eq!(page["pagination"]["limit"], 2);
	assert_eq!(page["pagination"]["hasNextPage"], true);
	assert_eq!(page["pagination"]["hasPreviousPage"], false);
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

	let (status, page2) = get_json!(
		app,
		&format!("/api/v2/signature-groups?limit=2&cursor={cursor}")
	);
	assert_eq!(status, 200);
	assert_eq!(page2["pagination"]["hasPreviousPage"], true);
	let second_names: Vec<String> = page2["data"]
		.as_array()
		.unwrap()
		.iter()
		.map(|g| g["name"].as_str().unwrap().to_string())
		.collect();

	for n in &second_names {
		assert!(
			!first_names.contains(n),
			"the second page must not repeat first-page rows: {n}"
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
async fn final_page_has_no_cursor() {
	let (_pg, db) = start_pg().await;
	seed_signature_groups(&db, 3).await;
	let app = test::init_service(build_app!(db)).await;

	// A large limit drains the whole table in one page.
	let (status, page) = get_json!(app, "/api/v2/signature-groups?limit=50");
	assert_eq!(status, 200);
	assert_eq!(page["pagination"]["hasNextPage"], false);
	assert!(page["pagination"]["nextCursor"].is_null());
}

#[actix_web::test]
async fn with_total_is_honored_on_signature_groups() {
	let (_pg, db) = start_pg().await;
	seed_signature_groups(&db, 4).await;
	let app = test::init_service(build_app!(db)).await;

	let (_, without) = get_json!(app, "/api/v2/signature-groups?limit=2");
	assert!(
		without["pagination"].get("totalItems").is_none()
			|| without["pagination"]["totalItems"].is_null(),
		"totalItems must be omitted by default"
	);

	let (_, with) = get_json!(app, "/api/v2/signature-groups?limit=2&withTotal=true");
	let total = with["pagination"]["totalItems"].as_u64().unwrap();
	assert!(
		total >= 4,
		"totalItems must count every seeded group, got {total}"
	);
}

#[actix_web::test]
async fn with_total_is_ignored_on_companies() {
	let (_pg, db) = start_pg().await;
	let app = test::init_service(build_app!(db)).await;

	let (status, page) = get_json!(app, "/api/v2/companies?withTotal=true");
	assert_eq!(status, 200);
	assert!(
		page["pagination"].get("totalItems").is_none()
			|| page["pagination"]["totalItems"].is_null(),
		"companies is write-hot; with_total must be ignored"
	);
}

#[actix_web::test]
async fn platforms_honor_with_total_and_clamp_limit() {
	let (_pg, db) = start_pg().await;
	seed_platforms(&db, 3).await;
	let app = test::init_service(build_app!(db)).await;

	// limit=9999 must clamp to 50, never 400.
	let (status, page) = get_json!(app, "/api/v2/platforms?limit=9999&withTotal=true");
	assert_eq!(status, 200);
	assert_eq!(page["pagination"]["limit"], 50);
	let total = page["pagination"]["totalItems"].as_u64().unwrap();
	assert!(
		total >= 3,
		"platform totalItems must count seeded rows, got {total}"
	);
}

#[actix_web::test]
async fn malformed_cursor_is_rejected_with_400() {
	let (_pg, db) = start_pg().await;
	let app = test::init_service(build_app!(db)).await;

	let (status, body) = get_json!(app, "/api/v2/signature-groups?cursor=%21%21%21not-base64");
	assert_eq!(status, 400);
	assert_eq!(body["code"], "malformed_cursor");
}

#[actix_web::test]
async fn cursor_from_another_list_is_rejected_with_restart_hint() {
	let (_pg, db) = start_pg().await;
	seed_signature_groups(&db, 4).await;
	seed_platforms(&db, 4).await;
	let app = test::init_service(build_app!(db)).await;

	let (_, sg) = get_json!(app, "/api/v2/signature-groups?limit=2");
	let sg_cursor = sg["pagination"]["nextCursor"].as_str().unwrap().to_string();

	// Replaying a signature-group cursor against the platforms list must 400 with
	// a structured restart hint, because the filter tags differ.
	let (status, body) = get_json!(app, &format!("/api/v2/platforms?cursor={sg_cursor}"));
	assert_eq!(status, 400);
	assert_eq!(body["code"], "cursor_filter_mismatch");
	assert_eq!(body["restart"], true);
}

#[actix_web::test]
async fn platforms_search_filters_by_name_and_paginates() {
	let (_pg, db) = start_pg().await;
	seed_platforms(&db, 5).await;
	let app = test::init_service(build_app!(db)).await;

	let (status, page) = get_json!(app, "/api/v2/platforms/search?query=zzz-plat&limit=2");
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
		.map(|p| p["name"].as_str().unwrap().to_string())
		.collect();

	let (status, page2) = get_json!(
		app,
		&format!("/api/v2/platforms/search?query=zzz-plat&limit=2&cursor={cursor}")
	);
	assert_eq!(status, 200);
	let second: Vec<String> = page2["data"]
		.as_array()
		.unwrap()
		.iter()
		.map(|p| p["name"].as_str().unwrap().to_string())
		.collect();
	for n in &second {
		assert!(!first.contains(n), "search pages must not overlap: {n}");
	}

	let (status, empty) = get_json!(app, "/api/v2/platforms/search?query=no-such-name&limit=10");
	assert_eq!(status, 200);
	assert_eq!(empty["data"].as_array().unwrap().len(), 0);

	let (status, body) = get_json!(app, "/api/v2/platforms/search?query=");
	assert_eq!(status, 400);
	assert_eq!(body["code"], "empty_query");
}

#[actix_web::test]
async fn signature_groups_search_filters_by_name() {
	let (_pg, db) = start_pg().await;
	seed_signature_groups(&db, 4).await;
	let app = test::init_service(build_app!(db)).await;

	let (status, page) = get_json!(app, "/api/v2/signature-groups/search?query=zzz-pg&limit=50");
	assert_eq!(status, 200);
	assert_eq!(page["data"].as_array().unwrap().len(), 4);
	assert_eq!(page["pagination"]["hasNextPage"], false);

	let (status, empty) = get_json!(app, "/api/v2/signature-groups/search?query=absent&limit=10");
	assert_eq!(status, 200);
	assert_eq!(empty["data"].as_array().unwrap().len(), 0);

	let (status, body) = get_json!(app, "/api/v2/signature-groups/search?query=");
	assert_eq!(status, 400);
	assert_eq!(body["code"], "empty_query");
}
