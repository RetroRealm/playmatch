//! Postgres + Redis backed tests for the cross-provider hash collision read
//! path. Two current games under two signature groups share a content hash; the
//! V1 single-winner resolver stays deterministic by display_priority while the
//! V2 ranked resolver surfaces both, element zero equal to the V1 winner. A
//! pure-addition import that retires nothing must still bust the cached winner
//! in both the V1 and V2 namespaces.
//! Require Docker.

use migration::{Migrator, MigratorTrait};
use redis::AsyncTypedCommands;
use redis::aio::MultiplexedConnection;
use sea_orm::{ConnectionTrait, Database, DbConn};
use service::cache::CacheStatus::{Cached, NonCached};
use service::db::game_file::get_game_files_from_game_id;
use service::identification::cache::{
	bust_identify_cache_for_hashes, crc_size_key,
	find_all_games_and_metadata_ids_by_crc_size_cached,
	find_all_games_and_metadata_ids_by_hash_cached,
};
use service::identification::identify_game_and_get_relations;
use service::model::{GameFileMatchSearch, GameMatchType};
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

// The Redump game id sorts below the No-Intro game id, so a tie that fell
// through to the id key would pick Redump. display_priority must override that
// and keep No-Intro the winner.
const GAME_NOINTRO: &str = "e5e5e5e5-e5e5-e5e5-e5e5-e5e5e5e5e5e5";
const GAME_REDUMP: &str = "0a0a0a0a-0a0a-0a0a-0a0a-0a0a0a0a0a0a";
const GF_NOINTRO: &str = "99999999-9999-9999-9999-999999999999";
const GF_REDUMP: &str = "07070707-0707-0707-0707-070707070707";

const SHARED_SHA1: &str = "da39a3ee5e6b4b0d3255bfef95601890afd80709";
const SHARED_CRC: &str = "1a2b3c4d";

// A third game added later that shares the crc AND size with the existing two,
// so it is a genuine co-hashed addition under the crc+size match basis.
const SG_LEGACY: &str = "1b1b1b1b-1b1b-1b1b-1b1b-1b1b1b1b1b1b";
const DF_LEGACY: &str = "a3a3a3a3-a3a3-a3a3-a3a3-a3a3a3a3a3a3";
const IMPORT_LEGACY: &str = "c5c5c5c5-c5c5-c5c5-c5c5-c5c5c5c5c5c5";
const GAME_LEGACY: &str = "0b0b0b0b-0b0b-0b0b-0b0b-0b0b0b0b0b0b";
const GF_LEGACY: &str = "08080808-0808-0808-0808-080808080808";

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
	let container = Redis::default().with_tag("7-alpine").start().await.unwrap();
	let port = container.get_host_port_ipv4(REDIS_PORT).await.unwrap();
	let url = format!("redis://127.0.0.1:{port}");
	let client = redis::Client::open(url).unwrap();
	let conn = client.get_multiplexed_async_connection().await.unwrap();
	(container, conn)
}

/// Two providers over one platform. No-Intro carries display_priority 10,
/// Redump 20, so No-Intro outranks Redump in both the V1 single winner and the
/// V2 ranking even though its game id sorts higher.
async fn seed_two_provider_collision(db: &DbConn) {
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
		VALUES ('{GAME_NOINTRO}', '{IMPORT_NOINTRO}', 'Shared Game (No-Intro)', true, '{IMPORT_NOINTRO}');
		INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id)
		VALUES ('{GAME_REDUMP}', '{IMPORT_REDUMP}', 'Shared Game (Redump)', true, '{IMPORT_REDUMP}');

		INSERT INTO game_file (id, game_id, file_name, file_size_in_bytes, sha1, crc, is_current, last_seen_dat_file_import_id)
		VALUES ('{GF_NOINTRO}', '{GAME_NOINTRO}', 'shared.nds', 1024, '{SHARED_SHA1}', '{SHARED_CRC}', true, '{IMPORT_NOINTRO}');
		INSERT INTO game_file (id, game_id, file_name, file_size_in_bytes, sha1, crc, is_current, last_seen_dat_file_import_id)
		VALUES ('{GF_REDUMP}', '{GAME_REDUMP}', 'shared.nds', 1024, '{SHARED_SHA1}', '{SHARED_CRC}', true, '{IMPORT_REDUMP}');
		"#
	);
	db.execute_unprepared(&sql).await.unwrap();
}

/// A third legacy-provider game that shares the crc and size with the existing
/// two, added after the collision is already cached. display_priority 30 ranks
/// it below both.
async fn seed_crc_only_addition(db: &DbConn) {
	let sql = format!(
		r#"
		INSERT INTO signature_group (id, name, display_priority) VALUES ('{SG_LEGACY}', 'DatsSite-Legacy', 30);
		INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id, latest_dat_file_import_id)
		VALUES ('{DF_LEGACY}', 'Legacy DS', '{PLAT}', '1.0', '{SG_LEGACY}', '{IMPORT_LEGACY}');
		INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at)
		VALUES ('{IMPORT_LEGACY}', '{DF_LEGACY}', 'legacy.dat', '1.0', 'cccccccc', now());

		INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id)
		VALUES ('{GAME_LEGACY}', '{IMPORT_LEGACY}', 'Crc Twin (Legacy)', true, '{IMPORT_LEGACY}');
		INSERT INTO game_file (id, game_id, file_name, file_size_in_bytes, crc, is_current, last_seen_dat_file_import_id)
		VALUES ('{GF_LEGACY}', '{GAME_LEGACY}', 'twin.nds', 1024, '{SHARED_CRC}', true, '{IMPORT_LEGACY}');
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

fn ranked_games(
	status: service::cache::CacheStatus<service::identification::cache::IdentifyEntryV2>,
) -> Vec<String> {
	let entry = match status {
		Cached(v) | NonCached(v) => v,
	};
	entry.games.iter().map(|g| g.game.id.to_string()).collect()
}

#[tokio::test]
async fn v1_single_winner_is_deterministic_by_display_priority() {
	let (_pg, db) = start_pg().await;
	let (_redis, mut redis) = start_redis().await;
	seed_two_provider_collision(&db).await;

	let search = GameFileMatchSearch {
		file_name: "shared.nds".to_string(),
		file_size: 1024,
		md5: None,
		sha1: Some(SHARED_SHA1.to_string()),
		sha256: None,
		crc: None,
	};

	let result = identify_game_and_get_relations(search, &mut redis, &db)
		.await
		.unwrap();
	let result = matched(&result);

	assert_eq!(result.game_match_type, GameMatchType::SHA1);
	let game = result.game.as_ref().expect("a sha1 hit must carry a game");
	assert_eq!(
		game.id.to_string(),
		GAME_NOINTRO,
		"the lower display_priority provider must win the tie, not the lower uuid"
	);
}

#[tokio::test]
async fn v2_ranked_resolver_returns_both_with_winner_first() {
	let (_pg, db) = start_pg().await;
	let (_redis, mut redis) = start_redis().await;
	seed_two_provider_collision(&db).await;

	let ranked = find_all_games_and_metadata_ids_by_hash_cached(
		SHARED_SHA1,
		GameMatchType::SHA1,
		&mut redis,
		&db,
	)
	.await
	.unwrap();
	let ids = ranked_games(ranked);

	assert_eq!(
		ids,
		vec![GAME_NOINTRO.to_string(), GAME_REDUMP.to_string()],
		"the ranked resolver must return both games with the V1 winner at element zero"
	);
}

#[tokio::test]
async fn v2_element_zero_equals_v1_winner_for_crc() {
	let (_pg, db) = start_pg().await;
	let (_redis, mut redis) = start_redis().await;
	seed_two_provider_collision(&db).await;

	let search = GameFileMatchSearch {
		file_name: "shared.nds".to_string(),
		file_size: 1024,
		md5: None,
		sha1: None,
		sha256: None,
		crc: Some(SHARED_CRC.to_string()),
	};
	let v1 = identify_game_and_get_relations(search, &mut redis, &db)
		.await
		.unwrap();
	let v1_winner = matched(&v1)
		.game
		.as_ref()
		.expect("a crc hit must carry a game")
		.id
		.to_string();

	let ranked =
		find_all_games_and_metadata_ids_by_crc_size_cached(SHARED_CRC, 1024, &mut redis, &db)
			.await
			.unwrap();
	let ids = ranked_games(ranked);

	assert_eq!(
		v1_winner, GAME_NOINTRO,
		"the crc single winner must follow the same display_priority ordering"
	);
	assert_eq!(
		ids.first().map(String::as_str),
		Some(GAME_NOINTRO),
		"crc element zero must equal the V1 crc winner"
	);
	assert_eq!(
		ids.len(),
		2,
		"both co-crc games must appear in the ranked crc union"
	);
}

#[tokio::test]
async fn pure_addition_busts_both_namespaces() {
	let (_pg, db) = start_pg().await;
	let (_redis, mut redis) = start_redis().await;
	seed_two_provider_collision(&db).await;

	// Warm both the V1 and V2 crc caches with the two-game union.
	let search = GameFileMatchSearch {
		file_name: "shared.nds".to_string(),
		file_size: 1024,
		md5: None,
		sha1: None,
		sha256: None,
		crc: Some(SHARED_CRC.to_string()),
	};
	identify_game_and_get_relations(search, &mut redis, &db)
		.await
		.unwrap();
	let warm =
		find_all_games_and_metadata_ids_by_crc_size_cached(SHARED_CRC, 1024, &mut redis, &db)
			.await
			.unwrap();
	wait_for_cache_key(
		&mut redis,
		"identify",
		"crc",
		&crc_size_key(SHARED_CRC, 1024),
	)
	.await;
	wait_for_cache_key(
		&mut redis,
		"identify_v2",
		"crc",
		&crc_size_key(SHARED_CRC, 1024),
	)
	.await;
	assert_eq!(ranked_games(warm).len(), 2);

	// A pure-addition import: a third game now shares the crc, nothing retired.
	seed_crc_only_addition(&db).await;
	let added_files = get_game_files_from_game_id(GAME_LEGACY.parse().unwrap(), &db)
		.await
		.unwrap();
	bust_identify_cache_for_hashes(&mut redis, &db, &added_files)
		.await
		.unwrap();

	assert!(
		cache_key_absent(
			&mut redis,
			"identify",
			"crc",
			&crc_size_key(SHARED_CRC, 1024)
		)
		.await,
		"the V1 crc winner must be busted after a co-hashed pure addition"
	);
	assert!(
		cache_key_absent(
			&mut redis,
			"identify_v2",
			"crc",
			&crc_size_key(SHARED_CRC, 1024)
		)
		.await,
		"the V2 crc union must be busted after a co-hashed pure addition"
	);

	// The next ranked read must now reflect all three crc-sharing games.
	let after =
		find_all_games_and_metadata_ids_by_crc_size_cached(SHARED_CRC, 1024, &mut redis, &db)
			.await
			.unwrap();
	assert_eq!(
		ranked_games(after),
		vec![
			GAME_NOINTRO.to_string(),
			GAME_REDUMP.to_string(),
			GAME_LEGACY.to_string()
		],
		"the rebuilt crc union must rank No-Intro, then Redump, then the newly added legacy game"
	);
}

/// Regression for the crc-only match bug: two games share a crc but differ in
/// size. The crc rung must pin the size, so a query resolves to the same-size
/// game only, and a size that matches neither is not a crc hit at all.
#[tokio::test]
async fn crc_rung_requires_matching_size() {
	let (_pg, db) = start_pg().await;
	let (_redis, mut redis) = start_redis().await;

	let sql = format!(
		r#"
		INSERT INTO platform (id, name) VALUES ('{PLAT}', 'Nintendo DS');
		INSERT INTO signature_group (id, name, display_priority) VALUES ('{SG_NOINTRO}', 'No-Intro', 10);
		INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id, latest_dat_file_import_id)
		VALUES ('{DF_NOINTRO}', 'No-Intro DS', '{PLAT}', '1.0', '{SG_NOINTRO}', '{IMPORT_NOINTRO}');
		INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at)
		VALUES ('{IMPORT_NOINTRO}', '{DF_NOINTRO}', 'nointro.dat', '1.0', 'aaaaaaaa', now());

		INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id)
		VALUES ('{GAME_NOINTRO}', '{IMPORT_NOINTRO}', 'Small Rom', true, '{IMPORT_NOINTRO}');
		INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id)
		VALUES ('{GAME_REDUMP}', '{IMPORT_NOINTRO}', 'Large Rom', true, '{IMPORT_NOINTRO}');

		INSERT INTO game_file (id, game_id, file_name, file_size_in_bytes, crc, is_current, last_seen_dat_file_import_id)
		VALUES ('{GF_NOINTRO}', '{GAME_NOINTRO}', 'small.nds', 1024, '{SHARED_CRC}', true, '{IMPORT_NOINTRO}');
		INSERT INTO game_file (id, game_id, file_name, file_size_in_bytes, crc, is_current, last_seen_dat_file_import_id)
		VALUES ('{GF_REDUMP}', '{GAME_REDUMP}', 'large.nds', 4096, '{SHARED_CRC}', true, '{IMPORT_NOINTRO}');
		"#
	);
	db.execute_unprepared(&sql).await.unwrap();

	let small =
		find_all_games_and_metadata_ids_by_crc_size_cached(SHARED_CRC, 1024, &mut redis, &db)
			.await
			.unwrap();
	assert_eq!(
		ranked_games(small),
		vec![GAME_NOINTRO.to_string()],
		"crc + size 1024 must resolve to the 1024-byte game only"
	);

	let large =
		find_all_games_and_metadata_ids_by_crc_size_cached(SHARED_CRC, 4096, &mut redis, &db)
			.await
			.unwrap();
	assert_eq!(
		ranked_games(large),
		vec![GAME_REDUMP.to_string()],
		"crc + size 4096 must resolve to the 4096-byte game only"
	);

	// A size that matches neither file is not a crc hit; with no other content key
	// the public identify path falls through to no match.
	let search = GameFileMatchSearch {
		file_name: "unrelated.nds".to_string(),
		file_size: 9999,
		md5: None,
		sha1: None,
		sha256: None,
		crc: Some(SHARED_CRC.to_string()),
	};
	let miss = identify_game_and_get_relations(search, &mut redis, &db)
		.await
		.unwrap();
	assert_eq!(
		matched(&miss).game_match_type,
		GameMatchType::NoMatch,
		"a crc with a non-matching size must not produce a crc match"
	);
}

async fn cache_key_absent(
	redis: &mut MultiplexedConnection,
	namespace: &str,
	segment: &str,
	identifier: &str,
) -> bool {
	let pattern = format!("*:{namespace}:{segment}:{identifier}");
	let keys = redis.keys(&pattern).await.unwrap();
	keys.is_empty()
}

async fn wait_for_cache_key(
	redis: &mut MultiplexedConnection,
	namespace: &str,
	segment: &str,
	identifier: &str,
) {
	let pattern = format!("*:{namespace}:{segment}:{identifier}");
	for _ in 0..100 {
		let keys = redis.keys(&pattern).await.unwrap();
		if !keys.is_empty() {
			return;
		}
		tokio::time::sleep(std::time::Duration::from_millis(50)).await;
	}
	panic!("cache key matching {pattern} never appeared; cache write did not land");
}
