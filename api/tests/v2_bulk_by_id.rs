//! Postgres backed tests for the v2 bulk get-by-id endpoints. They drive the
//! real actix v2 scope so routing, the literal `/{resource}/bulk` path winning
//! over `/{id}`, and the per-item envelope are exercised end to end. Require
//! Docker.

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
const COMPANY: &str = "33333333-3333-3333-3333-333333333333";
const PLAT: &str = "22222222-2222-2222-2222-222222222222";
const DF: &str = "a1a1a1a1-a1a1-a1a1-a1a1-a1a1a1a1a1a1";
const IMPORT: &str = "c3c3c3c3-c3c3-c3c3-c3c3-c3c3c3c3c3c3";
const GAME: &str = "e5e5e5e5-e5e5-e5e5-e5e5-e5e5e5e5e5e5";
const GAME_FILE: &str = "f6f6f6f6-f6f6-f6f6-f6f6-f6f6f6f6f6f6";
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

async fn start_redis() -> (ContainerAsync<Redis>, MultiplexedConnection) {
	let container = Redis::default().start().await.unwrap();
	let port = container.get_host_port_ipv4(REDIS_PORT).await.unwrap();
	let url = format!("redis://127.0.0.1:{port}");
	let client = redis::Client::open(url).unwrap();
	let conn = client.get_multiplexed_async_connection().await.unwrap();
	(container, conn)
}

async fn seed(db: &DatabaseConnection) {
	let sql = format!(
		r#"
		INSERT INTO signature_group (id, name) VALUES ('{SG}', 'No-Intro');
		INSERT INTO company (id, name) VALUES ('{COMPANY}', 'Nintendo');
		INSERT INTO platform (id, name, company_id) VALUES ('{PLAT}', 'Nintendo DS', '{COMPANY}');

		INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id, latest_dat_file_import_id)
		VALUES ('{DF}', 'Nintendo - Nintendo DS (Decrypted)', '{PLAT}', '20260617-122122', '{SG}', '{IMPORT}');

		INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at)
		VALUES ('{IMPORT}', '{DF}', 'Nintendo - Nintendo DS (Decrypted) (20260617-122122).dat', '20260617-122122', 'aaaaaaaa', '2026-06-18 12:00:00+00');

		INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id)
		VALUES ('{GAME}', '{IMPORT}', 'Pokemon - Diamant-Edition (Germany) (Rev 5)', true, '{IMPORT}');

		INSERT INTO game_file (id, game_id, file_name, is_current)
		VALUES ('{GAME_FILE}', '{GAME}', 'Pokemon - Diamant-Edition (Germany) (Rev 5).nds', true);
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
			.peer_addr("203.0.113.42:0".parse().unwrap())
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
async fn bulk_games_returns_found_and_not_found_per_item() {
	let (_pg, db) = start_pg().await;
	let (_redis, redis) = start_redis().await;
	seed(&db).await;
	let app = test::init_service(build_app!(db, redis)).await;

	let (status, _max, value) = post_json!(
		app,
		"/api/v2/games/bulk",
		&json!({ "ids": [GAME, MISSING] })
	);
	assert_eq!(
		status, 200,
		"the literal /games/bulk wins over /games/{{id}}"
	);

	assert_eq!(value["summary"]["total"], 2);
	assert_eq!(value["summary"]["found"], 1);
	assert_eq!(value["summary"]["notFound"], 1);

	let results = value["results"].as_array().unwrap();
	assert_eq!(results[0]["id"], GAME);
	assert_eq!(results[0]["status"], "ok");
	assert_eq!(
		results[0]["data"]["name"],
		"Pokemon - Diamant-Edition (Germany) (Rev 5)"
	);

	assert_eq!(results[1]["id"], MISSING);
	assert_eq!(results[1]["status"], "notFound");
	assert!(results[1]["data"].is_null());
}

#[actix_web::test]
async fn bulk_dedupes_ids_and_keeps_first_occurrence_order() {
	let (_pg, db) = start_pg().await;
	let (_redis, redis) = start_redis().await;
	seed(&db).await;
	let app = test::init_service(build_app!(db, redis)).await;

	let (status, _max, value) = post_json!(
		app,
		"/api/v2/games/bulk",
		&json!({ "ids": [MISSING, GAME, MISSING, GAME] })
	);
	assert_eq!(status, 200);

	assert_eq!(
		value["summary"]["total"], 2,
		"duplicates collapse to distinct ids"
	);
	let results = value["results"].as_array().unwrap();
	assert_eq!(results.len(), 2);
	assert_eq!(
		results[0]["id"], MISSING,
		"first occurrence order is preserved"
	);
	assert_eq!(results[1]["id"], GAME);
}

#[actix_web::test]
async fn bulk_platforms_companies_and_groups_resolve() {
	let (_pg, db) = start_pg().await;
	let (_redis, redis) = start_redis().await;
	seed(&db).await;
	let app = test::init_service(build_app!(db, redis)).await;

	let (status, _max, value) =
		post_json!(app, "/api/v2/platforms/bulk", &json!({ "ids": [PLAT] }));
	assert_eq!(status, 200);
	assert_eq!(value["summary"]["found"], 1);
	assert_eq!(value["results"][0]["data"]["name"], "Nintendo DS");

	let (status, _max, value) =
		post_json!(app, "/api/v2/companies/bulk", &json!({ "ids": [COMPANY] }));
	assert_eq!(status, 200);
	assert_eq!(value["summary"]["found"], 1);
	assert_eq!(value["results"][0]["data"]["name"], "Nintendo");

	let (status, _max, value) = post_json!(
		app,
		"/api/v2/signature-groups/bulk",
		&json!({ "ids": [SG, MISSING] })
	);
	assert_eq!(status, 200);
	assert_eq!(value["summary"]["found"], 1);
	assert_eq!(value["summary"]["notFound"], 1);
	assert_eq!(value["results"][0]["data"]["name"], "No-Intro");
}

#[actix_web::test]
async fn bulk_game_files_returns_found_and_not_found_per_item() {
	let (_pg, db) = start_pg().await;
	let (_redis, redis) = start_redis().await;
	seed(&db).await;
	let app = test::init_service(build_app!(db, redis)).await;

	let (status, _max, value) = post_json!(
		app,
		"/api/v2/game-files/bulk",
		&json!({ "ids": [GAME_FILE, MISSING] })
	);
	assert_eq!(
		status, 200,
		"the literal /game-files/bulk wins over /game-files/{{id}}"
	);

	assert_eq!(value["summary"]["total"], 2);
	assert_eq!(value["summary"]["found"], 1);
	assert_eq!(value["summary"]["notFound"], 1);

	let results = value["results"].as_array().unwrap();
	assert_eq!(results[0]["id"], GAME_FILE);
	assert_eq!(results[0]["status"], "ok");
	assert_eq!(
		results[0]["data"]["fileName"],
		"Pokemon - Diamant-Edition (Germany) (Rev 5).nds"
	);
	assert_eq!(results[0]["data"]["gameId"], GAME);

	assert_eq!(results[1]["id"], MISSING);
	assert_eq!(results[1]["status"], "notFound");
	assert!(results[1]["data"].is_null());
}

#[actix_web::test]
async fn bulk_dat_files_returns_found_not_found_and_dedupes() {
	let (_pg, db) = start_pg().await;
	let (_redis, redis) = start_redis().await;
	seed(&db).await;
	let app = test::init_service(build_app!(db, redis)).await;

	let (status, _max, value) = post_json!(
		app,
		"/api/v2/dat-files/bulk",
		&json!({ "ids": [DF, MISSING, DF] })
	);
	assert_eq!(status, 200);

	assert_eq!(
		value["summary"]["total"], 2,
		"duplicates collapse to distinct ids"
	);
	assert_eq!(value["summary"]["found"], 1);
	assert_eq!(value["summary"]["notFound"], 1);

	let results = value["results"].as_array().unwrap();
	assert_eq!(results[0]["id"], DF);
	assert_eq!(results[0]["status"], "ok");
	assert_eq!(
		results[0]["data"]["name"],
		"Nintendo - Nintendo DS (Decrypted)"
	);

	assert_eq!(results[1]["id"], MISSING);
	assert_eq!(results[1]["status"], "notFound");
	assert!(results[1]["data"].is_null());
}

#[actix_web::test]
async fn empty_batch_is_rejected() {
	let (_pg, db) = start_pg().await;
	let (_redis, redis) = start_redis().await;
	let app = test::init_service(build_app!(db, redis)).await;

	let (status, _max, value) = post_json!(app, "/api/v2/games/bulk", &json!({ "ids": [] }));
	assert_eq!(status, 400);
	assert_eq!(value["code"], "empty_batch");
	assert!(value["message"].is_string());
}

#[actix_web::test]
async fn over_cap_batch_is_rejected_with_header() {
	let (_pg, db) = start_pg().await;
	let (_redis, redis) = start_redis().await;
	let app = test::init_service(build_app!(db, redis)).await;

	let ids: Vec<String> = (0..101)
		.map(|i| format!("00000000-0000-0000-0000-{i:012}"))
		.collect();
	let (status, max_items, value) =
		post_json!(app, "/api/v2/companies/bulk", &json!({ "ids": ids }));

	assert_eq!(status, 400);
	assert_eq!(max_items.as_deref(), Some("100"));
	assert_eq!(value["code"], "batch_too_large");
	assert_eq!(value["limit"], 100);
	assert_eq!(value["received"], 101);
	assert!(value["message"].is_string());
}
