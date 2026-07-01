//! Postgres + Redis backed tests for the v2 bulk identify endpoints. They drive
//! the real actix v2 scope so routing, the scoped body limit and the per-item
//! envelope are exercised end to end. Require Docker.

use actix_web::web::Data;
use actix_web::{App, test};
use api::{
	PublicRouteFlags, public_api_governor_config, test_prometheus_metrics, versioned_api_scope,
};
use migration::{Migrator, MigratorTrait};
use redis::aio::MultiplexedConnection;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};
use serde_json::{Value, json};
use service::config::versions::ApiVersion;
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

async fn start_redis() -> (ContainerAsync<Redis>, MultiplexedConnection) {
	let container = Redis::default().start().await.unwrap();
	let port = container.get_host_port_ipv4(REDIS_PORT).await.unwrap();
	let url = format!("redis://127.0.0.1:{port}");
	let client = redis::Client::open(url).unwrap();
	let conn = client.get_multiplexed_async_connection().await.unwrap();
	(container, conn)
}

async fn seed_game(db: &DatabaseConnection) {
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

		INSERT INTO game_file (id, game_id, file_name, sha1, is_current, last_seen_dat_file_import_id)
		VALUES ('{GF}', '{GAME}', 'Pokemon - Diamant-Edition (Germany) (Rev 5).nds', '{SHA1}', true, '{IMPORT}');
		"#
	);
	db.execute_unprepared(&sql).await.unwrap();
}

macro_rules! build_app {
	($db:expr, $redis:expr) => {{
		let gov = public_api_governor_config();
		let prom = test_prometheus_metrics();
		App::new()
			.app_data(Data::new($db))
			.app_data(Data::new($redis))
			.service(versioned_api_scope(
				"/api/v2",
				ApiVersion::V2,
				PublicRouteFlags::default(),
				&gov,
				prom,
			))
	}};
}

macro_rules! post_json {
	($app:expr, $uri:expr, $body:expr) => {{
		let req = test::TestRequest::post()
			.uri($uri)
			.peer_addr("203.0.113.41:0".parse().unwrap())
			.set_json($body)
			.to_request();
		let resp = test::call_service(&$app, req).await;
		let status = resp.status().as_u16();
		let max_items = resp
			.headers()
			.get("X-Bulk-Max-Items")
			.map(|v| v.to_str().unwrap().to_string());
		let body = test::read_body(resp).await;
		let value: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
		(status, max_items, value)
	}};
}

#[actix_web::test]
async fn bulk_ids_returns_per_item_results_with_summary() {
	let (_pg, db) = start_pg().await;
	let (_redis, redis) = start_redis().await;
	seed_game(&db).await;
	let app = test::init_service(build_app!(db, redis)).await;

	let body = json!({
		"items": [
			{
				"fileName": "Pokemon - Diamant-Edition (Germany) (Rev 5).nds",
				"fileSize": 1024,
				"sha1": SHA1,
				"key": "hit"
			},
			{
				"fileName": "unknown.rom",
				"fileSize": 2048,
				"sha1": "0000000000000000000000000000000000000000",
				"key": "miss"
			},
			{
				"fileName": "bad.rom",
				"fileSize": 2048,
				"sha1": "not-hex",
				"key": "invalid"
			}
		]
	});

	let req = test::TestRequest::post()
		.uri("/api/v2/identify/bulk/ids")
		.peer_addr("203.0.113.41:0".parse().unwrap())
		.set_json(&body)
		.to_request();
	let resp = test::call_service(&app, req).await;
	assert_eq!(
		resp.status().as_u16(),
		200,
		"a mixed batch still returns a flat 200"
	);
	let cache_summary = resp
		.headers()
		.get("X-Cache-Summary")
		.map(|v| v.to_str().unwrap().to_string());
	assert!(
		cache_summary.is_some(),
		"bulk must emit an aggregate X-Cache-Summary header"
	);
	assert!(
		resp.headers().get("X-Cache").is_none(),
		"bulk must drop the single endpoint's X-Cache header"
	);
	let value: Value = test::read_body_json(resp).await;

	let summary = &value["summary"];
	assert_eq!(summary["total"], 3);
	assert_eq!(summary["succeeded"], 2);
	assert_eq!(summary["failed"], 1);
	assert_eq!(summary["matched"], 1);
	assert_eq!(summary["unmatched"], 1);

	let results = value["results"].as_array().unwrap();
	assert_eq!(results.len(), 3);

	let hit = &results[0];
	assert_eq!(hit["index"], 0);
	assert_eq!(hit["key"], "hit");
	assert_eq!(hit["status"], "ok");
	assert!(matches!(hit["cache"].as_str(), Some("HIT") | Some("MISS")));
	assert_eq!(hit["match"]["gameMatchType"], "SHA1");

	let miss = &results[1];
	assert_eq!(miss["status"], "ok");
	assert_eq!(miss["match"]["gameMatchType"], "NoMatch");

	let invalid = &results[2];
	assert_eq!(invalid["status"], "invalid");
	assert!(invalid["match"].is_null());
	assert_eq!(invalid["error"]["code"], "invalid_item");
}

#[actix_web::test]
async fn bulk_relations_orders_results_by_request_index() {
	let (_pg, db) = start_pg().await;
	let (_redis, redis) = start_redis().await;
	seed_game(&db).await;
	let app = test::init_service(build_app!(db, redis)).await;

	let items: Vec<Value> = (0..10)
		.map(|i| {
			json!({
				"fileName": format!("file-{i}.rom"),
				"fileSize": 100 + i,
				"sha1": "0000000000000000000000000000000000000000"
			})
		})
		.collect();
	let body = json!({ "items": items });

	let (status, _max, value) = post_json!(app, "/api/v2/identify/bulk/relations", &body);
	assert_eq!(status, 200);

	let results = value["results"].as_array().unwrap();
	let indices: Vec<u64> = results
		.iter()
		.map(|r| r["index"].as_u64().unwrap())
		.collect();
	assert_eq!(indices, (0..10).collect::<Vec<_>>());
	assert_eq!(value["summary"]["total"], 10);
}

#[actix_web::test]
async fn empty_batch_is_rejected() {
	let (_pg, db) = start_pg().await;
	let (_redis, redis) = start_redis().await;
	let app = test::init_service(build_app!(db, redis)).await;

	let (status, _max, value) =
		post_json!(app, "/api/v2/identify/bulk/ids", &json!({ "items": [] }));
	assert_eq!(status, 400);
	assert_eq!(value["code"], "empty_batch");
	assert!(value["message"].is_string());
}

#[actix_web::test]
async fn over_cap_batch_is_rejected_with_header() {
	let (_pg, db) = start_pg().await;
	let (_redis, redis) = start_redis().await;
	let app = test::init_service(build_app!(db, redis)).await;

	let items: Vec<Value> = (0..101)
		.map(|i| json!({ "fileName": format!("f-{i}.rom"), "fileSize": 1 }))
		.collect();
	let (status, max_items, value) = post_json!(
		app,
		"/api/v2/identify/bulk/relations",
		&json!({ "items": items })
	);

	assert_eq!(status, 400);
	assert_eq!(max_items.as_deref(), Some("100"));
	assert_eq!(value["code"], "batch_too_large");
	assert_eq!(value["limit"], 100);
	assert_eq!(value["received"], 101);
	assert!(value["message"].is_string());
}

#[actix_web::test]
async fn duplicate_key_is_rejected() {
	let (_pg, db) = start_pg().await;
	let (_redis, redis) = start_redis().await;
	let app = test::init_service(build_app!(db, redis)).await;

	let body = json!({
		"items": [
			{ "fileName": "a.rom", "fileSize": 1, "key": "dup" },
			{ "fileName": "b.rom", "fileSize": 1, "key": "dup" }
		]
	});
	let (status, _max, value) = post_json!(app, "/api/v2/identify/bulk/ids", &body);
	assert_eq!(status, 400);
	assert_eq!(value["code"], "duplicate_key");
}
