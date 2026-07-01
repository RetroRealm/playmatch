//! Postgres + Redis backed tests for the identify hash-precedence cascade.
//! Require Docker.

use migration::{Migrator, MigratorTrait};
use redis::AsyncTypedCommands;
use redis::aio::MultiplexedConnection;
use sea_orm::{ConnectionTrait, Database, DbConn};
use service::cache::CacheStatus::{Cached, NonCached};
use service::identification::identify_game_and_get_relations;
use service::model::{GameFileMatchSearch, GameMatchType};
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::redis::{REDIS_PORT, Redis};
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use testcontainers_modules::testcontainers::{ContainerAsync, ImageExt};

const SG: &str = "11111111-1111-1111-1111-111111111111";
const PLAT: &str = "22222222-2222-2222-2222-222222222222";
const DF: &str = "a1a1a1a1-a1a1-a1a1-a1a1-a1a1a1a1a1a1";
const IMPORT: &str = "c3c3c3c3-c3c3-c3c3-c3c3-c3c3c3c3c3c3";

const GAME_A: &str = "e5e5e5e5-e5e5-e5e5-e5e5-e5e5e5e5e5e5";
const GAME_B: &str = "f6f6f6f6-f6f6-f6f6-f6f6-f6f6f6f6f6f6";
const GF_A: &str = "99999999-9999-9999-9999-999999999999";
const GF_B: &str = "07070707-0707-0707-0707-070707070707";

const MD5_X: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const CRC_Y: &str = "1a2b3c4d";

const ONLY_NS_GF: &str = "0b0b0b0b-0b0b-0b0b-0b0b-0b0b0b0b0b0b";
const ONLY_NS_GAME: &str = "0c0c0c0c-0c0c-0c0c-0c0c-0c0c0c0c0c0c";
const ONLY_NS_NAME: &str = "Only Name And Size (Region).rom";
const ONLY_NS_SIZE: i64 = 4096;
const NON_MATCHING_SHA1: &str = "0000000000000000000000000000000000000000";

async fn start_pg() -> (ContainerAsync<Postgres>, DbConn) {
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
	// The identify cache read path uses GETEX, which the testcontainers default
	// redis:5.0 image does not implement. Pin a 7.x image so a cache hit is
	// actually served instead of silently degrading to a database read.
	let container = Redis::default().with_tag("7-alpine").start().await.unwrap();
	let port = container.get_host_port_ipv4(REDIS_PORT).await.unwrap();
	let url = format!("redis://127.0.0.1:{port}");
	let client = redis::Client::open(url).unwrap();
	let conn = client.get_multiplexed_async_connection().await.unwrap();
	(container, conn)
}

async fn seed_base(db: &DbConn) {
	let sql = format!(
		r#"
		INSERT INTO signature_group (id, name) VALUES ('{SG}', 'No-Intro');
		INSERT INTO platform (id, name) VALUES ('{PLAT}', 'Nintendo DS');

		INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id, latest_dat_file_import_id)
		VALUES ('{DF}', 'Nintendo - Nintendo DS (Decrypted)', '{PLAT}', '20260617-122122', '{SG}', '{IMPORT}');

		INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at)
		VALUES ('{IMPORT}', '{DF}', 'Nintendo - Nintendo DS (Decrypted) (20260617-122122).dat', '20260617-122122', 'aaaaaaaa', '2026-06-18 12:00:00+00');
		"#
	);
	db.execute_unprepared(&sql).await.unwrap();
}

/// Two distinct current games whose files collide across hash types: game A is
/// reachable only by md5 X, game B only by crc Y. A search carrying both must
/// resolve game A because MD5 outranks CRC in the cascade.
async fn seed_md5_crc_conflict(db: &DbConn) {
	let sql = format!(
		r#"
		INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id)
		VALUES ('{GAME_A}', '{IMPORT}', 'Game Reachable By Md5', true, '{IMPORT}');
		INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id)
		VALUES ('{GAME_B}', '{IMPORT}', 'Game Reachable By Crc', true, '{IMPORT}');

		INSERT INTO game_file (id, game_id, file_name, file_size_in_bytes, md5, is_current, last_seen_dat_file_import_id)
		VALUES ('{GF_A}', '{GAME_A}', 'reachable-by-md5.nds', 1024, '{MD5_X}', true, '{IMPORT}');
		INSERT INTO game_file (id, game_id, file_name, file_size_in_bytes, crc, is_current, last_seen_dat_file_import_id)
		VALUES ('{GF_B}', '{GAME_B}', 'reachable-by-crc.nds', 2048, '{CRC_Y}', true, '{IMPORT}');
		"#
	);
	db.execute_unprepared(&sql).await.unwrap();
}

/// A single current file carrying no hashes, matchable only by file name and
/// exact size.
async fn seed_name_size_only(db: &DbConn) {
	let sql = format!(
		r#"
		INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id)
		VALUES ('{ONLY_NS_GAME}', '{IMPORT}', 'Only Name And Size', true, '{IMPORT}');

		INSERT INTO game_file (id, game_id, file_name, file_size_in_bytes, is_current, last_seen_dat_file_import_id)
		VALUES ('{ONLY_NS_GF}', '{ONLY_NS_GAME}', '{ONLY_NS_NAME}', {ONLY_NS_SIZE}, true, '{IMPORT}');
		"#
	);
	db.execute_unprepared(&sql).await.unwrap();
}

fn matched(
	result: &service::cache::CacheStatus<service::model::GameAndRelationMatchResult>,
) -> &service::model::GameAndRelationMatchResult {
	match result {
		Cached(inner) | NonCached(inner) => inner,
	}
}

#[tokio::test]
async fn md5_outranks_crc_when_a_search_carries_both() {
	let (_pg, db) = start_pg().await;
	let (_redis, mut redis) = start_redis().await;
	seed_base(&db).await;
	seed_md5_crc_conflict(&db).await;

	let search = GameFileMatchSearch {
		file_name: "ambiguous.nds".to_string(),
		file_size: 9999,
		md5: Some(MD5_X.to_string()),
		sha1: None,
		sha256: None,
		crc: Some(CRC_Y.to_string()),
	};

	let result = identify_game_and_get_relations(search, &mut redis, &db)
		.await
		.unwrap();
	let result = matched(&result);

	assert_eq!(
		result.game_match_type,
		GameMatchType::MD5,
		"md5 must win over crc because it sits higher in the cascade"
	);
	let game = result
		.game
		.as_ref()
		.expect("a hit must carry the matched game");
	assert_eq!(
		game.id.to_string(),
		GAME_A,
		"the md5 path must resolve game A, not the crc-only game B"
	);
}

#[tokio::test]
async fn falls_through_to_filename_and_size_when_hashes_miss() {
	let (_pg, db) = start_pg().await;
	let (_redis, mut redis) = start_redis().await;
	seed_base(&db).await;
	seed_name_size_only(&db).await;

	let search = GameFileMatchSearch {
		file_name: ONLY_NS_NAME.to_string(),
		file_size: ONLY_NS_SIZE,
		md5: None,
		sha1: Some(NON_MATCHING_SHA1.to_string()),
		sha256: None,
		crc: None,
	};

	let result = identify_game_and_get_relations(search, &mut redis, &db)
		.await
		.unwrap();
	let result = matched(&result);

	assert_eq!(
		result.game_match_type,
		GameMatchType::FileNameAndSize,
		"a non-matching sha1 must fall through to a name+size resolution"
	);
	let game = result
		.game
		.as_ref()
		.expect("a name+size hit must carry the matched game");
	assert_eq!(
		game.id.to_string(),
		ONLY_NS_GAME,
		"the name+size path must resolve the only file carrying that name and size"
	);
}

#[tokio::test]
async fn second_identical_lookup_resolves_from_cache() {
	let (_pg, db) = start_pg().await;
	let (_redis, mut redis) = start_redis().await;
	seed_base(&db).await;
	seed_md5_crc_conflict(&db).await;

	let search = GameFileMatchSearch {
		file_name: "ambiguous.nds".to_string(),
		file_size: 9999,
		md5: Some(MD5_X.to_string()),
		sha1: None,
		sha256: None,
		crc: None,
	};

	let first = identify_game_and_get_relations(search.clone(), &mut redis, &db)
		.await
		.unwrap();
	assert!(
		matches!(first, NonCached(_)),
		"the first lookup must miss the cache and read from the database"
	);

	wait_for_cache_key(&mut redis, "md5", MD5_X).await;

	let second = identify_game_and_get_relations(search, &mut redis, &db)
		.await
		.unwrap();
	assert!(
		matches!(second, Cached(_)),
		"the second identical lookup must resolve from the warmed cache"
	);
	assert_eq!(
		matched(&second).game_match_type,
		GameMatchType::MD5,
		"the cached result must carry the same match type as the live lookup"
	);
}

async fn wait_for_cache_key(redis: &mut MultiplexedConnection, segment: &str, identifier: &str) {
	let pattern = format!("*:identify:{segment}:{identifier}");
	for _ in 0..100 {
		let keys = redis.keys(&pattern).await.unwrap();
		if !keys.is_empty() {
			return;
		}
		tokio::time::sleep(std::time::Duration::from_millis(50)).await;
	}
	panic!("cache key matching {pattern} never appeared; cache write did not land");
}
