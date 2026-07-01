//! Postgres + Redis backed tests for the cross-provider hash collision surface.
//! Two current games under two signature groups share a content hash. The v2
//! identify response carries the primary plus the co-hashed sibling in
//! additionalMatches, while the v1 response carries only the primary. A
//! pure-addition that retires nothing must still bust the cached union so the
//! next read reflects the newly co-hashed game.
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
use service::db::game_file::get_game_files_from_game_id;
use service::identification::cache::bust_identify_cache_for_hashes;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::redis::{REDIS_PORT, Redis};
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use testcontainers_modules::testcontainers::{ContainerAsync, ImageExt};

const PLAT: &str = "22222222-2222-2222-2222-222222222222";
const SG_NOINTRO: &str = "11111111-1111-1111-1111-111111111111";
const SG_REDUMP: &str = "1a1a1a1a-1a1a-1a1a-1a1a-1a1a1a1a1a1a";
const DF_NOINTRO: &str = "a1a1a1a1-a1a1-a1a1-a1a1-a1a1a1a1a1a1";
const DF_REDUMP: &str = "a2a2a2a2-a2a2-a2a2-a2a2-a2a2a2a2a2a2";
const IMPORT_NOINTRO: &str = "c3c3c3c3-c3c3-c3c3-c3c3-c3c3c3c3c3c3";
const IMPORT_REDUMP: &str = "c4c4c4c4-c4c4-c4c4-c4c4-c4c4c4c4c4c4";

const GAME_NOINTRO: &str = "e5e5e5e5-e5e5-e5e5-e5e5-e5e5e5e5e5e5";
const GAME_REDUMP: &str = "0a0a0a0a-0a0a-0a0a-0a0a-0a0a0a0a0a0a";

const SHARED_SHA1: &str = "da39a3ee5e6b4b0d3255bfef95601890afd80709";

const SG_LEGACY: &str = "1b1b1b1b-1b1b-1b1b-1b1b-1b1b1b1b1b1b";
const DF_LEGACY: &str = "a3a3a3a3-a3a3-a3a3-a3a3-a3a3a3a3a3a3";
const IMPORT_LEGACY: &str = "c5c5c5c5-c5c5-c5c5-c5c5-c5c5c5c5c5c5";
const GAME_LEGACY: &str = "0b0b0b0b-0b0b-0b0b-0b0b-0b0b0b0b0b0b";
const GF_LEGACY: &str = "08080808-0808-0808-0808-080808080808";

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
	let container = Redis::default().with_tag("7-alpine").start().await.unwrap();
	let port = container.get_host_port_ipv4(REDIS_PORT).await.unwrap();
	let url = format!("redis://127.0.0.1:{port}");
	let client = redis::Client::open(url).unwrap();
	let conn = client.get_multiplexed_async_connection().await.unwrap();
	(container, conn)
}

/// Two providers over one platform, each with a current game whose single file
/// carries the same sha1. No-Intro has display_priority 10 (the winner), Redump
/// 20 (the sibling).
async fn seed_collision(db: &DatabaseConnection) {
	let sql = format!(
		r#"
		INSERT INTO platform (id, name) VALUES ('{PLAT}', 'Nintendo DS');

		INSERT INTO signature_group (id, name, display_priority) VALUES ('{SG_NOINTRO}', 'No-Intro', 10);
		INSERT INTO signature_group (id, name, display_priority) VALUES ('{SG_REDUMP}', 'Redump', 20);

		INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id, latest_dat_file_import_id)
		VALUES ('{DF_NOINTRO}', 'No-Intro DS', '{PLAT}', '1.0', '{SG_NOINTRO}', '{IMPORT_NOINTRO}');
		INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id, latest_dat_file_import_id)
		VALUES ('{DF_REDUMP}', 'Redump DS', '{PLAT}', '1.0', '{SG_REDUMP}', '{IMPORT_REDUMP}');

		INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at)
		VALUES ('{IMPORT_NOINTRO}', '{DF_NOINTRO}', 'nointro.dat', '1.0', 'aaaaaaaa', now());
		INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at)
		VALUES ('{IMPORT_REDUMP}', '{DF_REDUMP}', 'redump.dat', '1.0', 'bbbbbbbb', now());

		INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id)
		VALUES ('{GAME_NOINTRO}', '{IMPORT_NOINTRO}', 'Shared (No-Intro)', true, '{IMPORT_NOINTRO}');
		INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id)
		VALUES ('{GAME_REDUMP}', '{IMPORT_REDUMP}', 'Shared (Redump)', true, '{IMPORT_REDUMP}');

		INSERT INTO game_file (id, game_id, file_name, file_size_in_bytes, sha1, is_current, last_seen_dat_file_import_id)
		VALUES ('99999999-9999-9999-9999-999999999999', '{GAME_NOINTRO}', 'shared.nds', 1024, '{SHARED_SHA1}', true, '{IMPORT_NOINTRO}');
		INSERT INTO game_file (id, game_id, file_name, file_size_in_bytes, sha1, is_current, last_seen_dat_file_import_id)
		VALUES ('07070707-0707-0707-0707-070707070707', '{GAME_REDUMP}', 'shared.nds', 1024, '{SHARED_SHA1}', true, '{IMPORT_REDUMP}');
		"#
	);
	db.execute_unprepared(&sql).await.unwrap();
}

/// A third game added later that also carries the shared sha1, simulating a
/// pure-addition import where nothing is retired.
async fn seed_third_collision(db: &DatabaseConnection) {
	let sql = format!(
		r#"
		INSERT INTO signature_group (id, name, display_priority) VALUES ('{SG_LEGACY}', 'DatsSite-Legacy', 30);
		INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id, latest_dat_file_import_id)
		VALUES ('{DF_LEGACY}', 'Legacy DS', '{PLAT}', '1.0', '{SG_LEGACY}', '{IMPORT_LEGACY}');
		INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at)
		VALUES ('{IMPORT_LEGACY}', '{DF_LEGACY}', 'legacy.dat', '1.0', 'cccccccc', now());

		INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id)
		VALUES ('{GAME_LEGACY}', '{IMPORT_LEGACY}', 'Shared (Legacy)', true, '{IMPORT_LEGACY}');
		INSERT INTO game_file (id, game_id, file_name, file_size_in_bytes, sha1, is_current, last_seen_dat_file_import_id)
		VALUES ('{GF_LEGACY}', '{GAME_LEGACY}', 'shared.nds', 1024, '{SHARED_SHA1}', true, '{IMPORT_LEGACY}');
		"#
	);
	db.execute_unprepared(&sql).await.unwrap();
}

macro_rules! build_app {
	($db:expr, $redis:expr, $version:expr) => {{
		let gov = public_api_governor_config();
		let prom = test_prometheus_metrics();
		App::new()
			.app_data(Data::new($db))
			.app_data(Data::new($redis))
			.service(versioned_api_scope(
				"/api",
				$version,
				PublicRouteFlags::default(),
				&gov,
				prom,
			))
	}};
}

macro_rules! identify {
	($app:expr, $query:expr, $ip:expr) => {{
		let req = test::TestRequest::get()
			.uri(&format!("/api/identify/relations?{}", $query))
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
async fn v2_surfaces_sibling_in_additional_matches_while_v1_carries_only_primary() {
	let (_pg, db) = start_pg().await;
	let (_redis, redis) = start_redis().await;
	seed_collision(&db).await;

	let v2 = test::init_service(build_app!(db.clone(), redis.clone(), ApiVersion::V2)).await;
	let v1 = test::init_service(build_app!(db.clone(), redis.clone(), ApiVersion::V1)).await;

	let query = format!("fileName=shared.nds&fileSize=1024&sha1={SHARED_SHA1}");

	let (status, _cache, v2_body) = identify!(v2, query, "203.0.113.20");
	assert_eq!(status, 200);
	assert_eq!(
		v2_body["game"]["id"], GAME_NOINTRO,
		"the v2 primary must be the display_priority winner, got: {v2_body}"
	);
	let additional = v2_body["additionalMatches"]
		.as_array()
		.expect("the v2 response must carry an additionalMatches array on a collision");
	assert_eq!(
		additional.len(),
		1,
		"exactly the one co-hashed sibling must surface, got: {v2_body}"
	);
	let sibling = &additional[0];
	assert_eq!(
		sibling["game"]["id"], GAME_REDUMP,
		"the surfaced sibling must be the lower-priority Redump game, got: {v2_body}"
	);
	assert_eq!(
		sibling["platform"]["id"], PLAT,
		"the sibling must carry its own platform, got: {v2_body}"
	);
	assert_eq!(
		sibling["signatureGroup"]["id"], SG_REDUMP,
		"the sibling must carry its own Redump signature group, not the primary's, got: {v2_body}"
	);
	assert_eq!(
		sibling["datFile"]["id"], DF_REDUMP,
		"the sibling must carry its own Redump dat file, not the primary's, got: {v2_body}"
	);

	let (status, _cache, v1_body) = identify!(v1, query, "203.0.113.21");
	assert_eq!(status, 200);
	assert_eq!(
		v1_body["game"]["id"], GAME_NOINTRO,
		"the v1 primary must equal the v2 primary, got: {v1_body}"
	);
	assert!(
		v1_body.get("additionalMatches").is_none(),
		"the v1 response must never carry additionalMatches, got: {v1_body}"
	);
}

#[actix_web::test]
async fn pure_addition_bust_lets_the_next_read_see_the_new_sibling() {
	let (_pg, db) = start_pg().await;
	let (_redis, redis) = start_redis().await;
	seed_collision(&db).await;

	let v2 = test::init_service(build_app!(db.clone(), redis.clone(), ApiVersion::V2)).await;
	let query = format!("fileName=shared.nds&fileSize=1024&sha1={SHARED_SHA1}");

	// Warm the v2 union, then poll until the cache write lands.
	let (_, _, warm) = identify!(v2, query, "203.0.113.30");
	assert_eq!(
		warm["additionalMatches"].as_array().map(|a| a.len()),
		Some(1),
		"the warmed union must hold one sibling, got: {warm}"
	);

	let mut warmed = false;
	for attempt in 0..50 {
		let ip = format!("198.51.100.{}", attempt + 1);
		let (_, cache, _) = identify!(v2, query, ip);
		if cache.as_deref() == Some("HIT") {
			warmed = true;
			break;
		}
		tokio::time::sleep(std::time::Duration::from_millis(100)).await;
	}
	assert!(
		warmed,
		"the v2 union must warm into a cache hit before the bust"
	);

	// A pure-addition import: a third co-hashed game, nothing retired.
	seed_third_collision(&db).await;
	let mut redis_conn = redis.clone();
	let added_files = get_game_files_from_game_id(GAME_LEGACY.parse().unwrap(), &db)
		.await
		.unwrap();
	bust_identify_cache_for_hashes(&mut redis_conn, &db, &added_files)
		.await
		.unwrap();

	// The next read must rebuild and surface both siblings end to end.
	let (status, _cache, after) = identify!(v2, query, "203.0.113.31");
	assert_eq!(status, 200);
	let additional = after["additionalMatches"]
		.as_array()
		.expect("the rebuilt union must carry additionalMatches");
	let sibling_ids: std::collections::HashSet<String> = additional
		.iter()
		.map(|entry| entry["game"]["id"].as_str().unwrap().to_string())
		.collect();
	let expected: std::collections::HashSet<String> =
		[GAME_REDUMP.to_string(), GAME_LEGACY.to_string()]
			.into_iter()
			.collect();
	assert_eq!(
		sibling_ids, expected,
		"after the pure-addition bust the union must surface exactly the Redump and legacy siblings, got: {after}"
	);
}
