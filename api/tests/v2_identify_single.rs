//! Postgres + Redis backed tests for the single v2 identify endpoint. They drive
//! the real actix v2 scope so the hash cascade, the no-match outcome and the
//! identify cache are exercised end to end, and they pin the camelCase wire shape
//! of the v2 match result.
//! Require Docker.

use actix_web::web::Data;
use actix_web::{App, test};
use api::{
	PublicRouteFlags, public_api_governor_config, test_prometheus_metrics, versioned_api_scope,
};
use migration::{Migrator, MigratorTrait};
use redis::aio::MultiplexedConnection;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};
use serde_json::Value;
use service::config::versions::ApiVersion;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::redis::{REDIS_PORT, Redis};
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use testcontainers_modules::testcontainers::{ContainerAsync, ImageExt};

const SG_ID: &str = "11111111-1111-1111-1111-111111111111";
const PLATFORM_ID: &str = "22222222-2222-2222-2222-222222222222";
const DAT_ID: &str = "33333333-3333-3333-3333-333333333333";
const IMPORT_ID: &str = "44444444-4444-4444-4444-444444444444";
const GAME_ID: &str = "66666666-6666-6666-6666-666666666666";
const GAME_FILE_ID: &str = "77777777-7777-7777-7777-777777777777";

const FILE_NAME: &str = "some-game.nes";
const FILE_SIZE: i64 = 1024;
const SHA1: &str = "da39a3ee5e6b4b0d3255bfef95601890afd80709";
const MD5: &str = "d41d8cd98f00b204e9800998ecf8427e";
const CRC: &str = "1a2b3c4d";
const UNKNOWN_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

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
	// 7-alpine so the identify read path's GETEX is available; the module default
	// (redis 5.0) predates GETEX and would surface every cached entry as a miss.
	let container = Redis::default().with_tag("7-alpine").start().await.unwrap();
	let port = container.get_host_port_ipv4(REDIS_PORT).await.unwrap();
	let url = format!("redis://127.0.0.1:{port}");
	let client = redis::Client::open(url).unwrap();
	let conn = client.get_multiplexed_async_connection().await.unwrap();
	(container, conn)
}

/// One dat file with a single import, one current game and one current game file
/// carrying a known sha1, md5 and crc, plus a presence row for the file.
async fn seed(db: &DatabaseConnection) {
	let setup = format!(
		r#"
        INSERT INTO signature_group (id, name) VALUES ('{SG_ID}', 'No-Intro');
        INSERT INTO platform (id, name) VALUES ('{PLATFORM_ID}', 'NES');
        INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id, latest_dat_file_import_id)
          VALUES ('{DAT_ID}', 'nes-dat', '{PLATFORM_ID}', '1.0', '{SG_ID}', '{IMPORT_ID}');
        INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at)
          VALUES ('{IMPORT_ID}', '{DAT_ID}', 'import', '1.0', 'd41d8cd98f00b204e9800998ecf8427e', now());
        INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id)
          VALUES ('{GAME_ID}', '{IMPORT_ID}', 'Some Game', true, '{IMPORT_ID}');
        INSERT INTO game_file (id, game_id, file_name, file_size_in_bytes, sha1, md5, crc, is_current, last_seen_dat_file_import_id)
          VALUES ('{GAME_FILE_ID}', '{GAME_ID}', '{FILE_NAME}', {FILE_SIZE}, '{SHA1}', '{MD5}', '{CRC}', true, '{IMPORT_ID}');
        INSERT INTO game_file_presence (id, game_file_id, dat_file_import_id)
          VALUES (gen_random_uuid(), '{GAME_FILE_ID}', '{IMPORT_ID}');
    "#
	);
	db.execute_unprepared(&setup).await.unwrap();
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

macro_rules! identify {
	($app:expr, $query:expr) => {{ identify!($app, $query, "203.0.113.9") }};
	($app:expr, $query:expr, $ip:expr) => {{
		let req = test::TestRequest::get()
			.uri(&format!("/api/v2/identify/relations?{}", $query))
			.peer_addr(format!("{}:0", $ip).parse().unwrap())
			.to_request();
		let resp = test::call_service(&$app, req).await;
		let status = resp.status().as_u16();
		let cache = resp
			.headers()
			.get("X-Cache")
			.map(|v| v.to_str().unwrap().to_string());
		let body = test::read_body(resp).await;
		let json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
		(status, cache, json)
	}};
}

#[actix_web::test]
async fn identify_by_sha1_returns_the_seeded_game() {
	let (_pg, db) = start_pg().await;
	let (_redis, redis) = start_redis().await;
	seed(&db).await;
	let app = test::init_service(build_app!(db, redis)).await;

	let (status, _cache, body) = identify!(
		app,
		format!("fileName={FILE_NAME}&fileSize={FILE_SIZE}&sha1={SHA1}")
	);
	assert_eq!(status, 200);
	assert_eq!(
		body["gameMatchType"], "SHA1",
		"a sha1 hit must report the SHA1 match type on the camelCase wire field, got: {body}"
	);
	assert_eq!(
		body["game"]["id"], GAME_ID,
		"the matched payload must carry the seeded game id, got: {body}"
	);
}

#[actix_web::test]
async fn identify_unknown_hash_returns_no_match() {
	let (_pg, db) = start_pg().await;
	let (_redis, redis) = start_redis().await;
	seed(&db).await;
	let app = test::init_service(build_app!(db, redis)).await;

	let (status, _cache, body) = identify!(
		app,
		format!("fileName=unknown.rom&fileSize=2048&sha256={UNKNOWN_SHA256}")
	);
	assert_eq!(status, 200);
	assert_eq!(
		body["gameMatchType"], "NoMatch",
		"an unknown hash must be a normal NoMatch result, got: {body}"
	);
	assert!(
		body.get("game").is_none(),
		"a no-match result must omit the game field entirely, got: {body}"
	);
}

#[actix_web::test]
async fn identify_cache_status_flips_from_miss_to_hit() {
	let (_pg, db) = start_pg().await;
	let (_redis, redis) = start_redis().await;
	seed(&db).await;
	let app = test::init_service(build_app!(db, redis)).await;

	let query = format!("fileName={FILE_NAME}&fileSize={FILE_SIZE}&sha1={SHA1}");

	let (first_status, first_cache, _) = identify!(app, query, "203.0.113.10");
	assert_eq!(first_status, 200);
	assert_eq!(
		first_cache.as_deref(),
		Some("MISS"),
		"the first identify of a hash must miss the cache"
	);

	// The identify cache is written behind the response on a spawned task, so the
	// flip to a hit is eventually consistent. Poll a repeated call until the write
	// lands rather than racing it on the very next request. Each poll comes from a
	// distinct client IP so the per-client rate limiter never throttles the wait.
	let mut last_cache = first_cache;
	for attempt in 0..50 {
		let ip = format!("198.51.100.{}", attempt + 1);
		let (status, cache, _) = identify!(app, query, ip);
		assert_eq!(status, 200);
		last_cache = cache;
		if last_cache.as_deref() == Some("HIT") {
			break;
		}
		tokio::time::sleep(std::time::Duration::from_millis(100)).await;
	}
	assert_eq!(
		last_cache.as_deref(),
		Some("HIT"),
		"a repeated identify of the same hash must hit the cache once the write lands"
	);
}
