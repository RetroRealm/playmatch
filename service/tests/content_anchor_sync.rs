//! Postgres + Redis backed tests for the content-anchor reconcile feature
//! (Part B). They exercise anchor formation over full SHA1-set equality, the
//! secondary-hash veto, survivorship (Manual over Automatic, never downgrade a
//! Manual, both-Manual-disagree left untouched), reconcile idempotency, and the
//! conflict-safe find-or-create.
//! Require Docker.

use migration::{Migrator, MigratorTrait};
use redis::aio::MultiplexedConnection;
use sea_orm::prelude::Uuid;
use sea_orm::{ConnectionTrait, Database, DbConn, EntityTrait};
use service::db::game::get_game_by_id;
use service::db::game_file::{
	assign_content_anchor_for_game, find_games_by_content_anchor, find_multi_member_anchor_ids,
};
use service::providers::content_anchor::run_content_anchor_reconcile_wave;
use service::providers::{MetadataProvider, ProviderRegistry};
use std::sync::Arc;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::redis::{REDIS_PORT, Redis};
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use testcontainers_modules::testcontainers::{ContainerAsync, ImageExt};

use entity::sea_orm_active_enums::{AutomaticMatchReasonEnum, MatchTypeEnum, MetadataProviderEnum};
use entity::signature_metadata_mapping;

const PLAT: &str = "22222222-2222-2222-2222-222222222222";
const SG_NOINTRO: &str = "11111111-1111-1111-1111-111111111111";
const SG_REDUMP: &str = "1a1a1a1a-1a1a-1a1a-1a1a-1a1a1a1a1a1a";
const DF_NOINTRO: &str = "a1a1a1a1-a1a1-a1a1-a1a1-a1a1a1a1a1a1";
const DF_REDUMP: &str = "a2a2a2a2-a2a2-a2a2-a2a2-a2a2a2a2a2a2";
const IMPORT_NOINTRO: &str = "c3c3c3c3-c3c3-c3c3-c3c3-c3c3c3c3c3c3";
const IMPORT_REDUMP: &str = "c4c4c4c4-c4c4-c4c4-c4c4-c4c4c4c4c4c4";

const SHA1_TRACK_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SHA1_TRACK_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const SHA1_TRACK_C: &str = "cccccccccccccccccccccccccccccccccccccccc";

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

/// Minimal provider whose only job is to carry a Redis handle into the reconcile
/// wave, which calls `registry.first().redis_conn()`. None of the matching entry
/// points run in these tests.
struct StubProvider {
	redis: MultiplexedConnection,
}

#[async_trait::async_trait]
impl MetadataProvider for StubProvider {
	fn provider_label(&self) -> &'static str {
		"igdb"
	}
	fn provider_enum(&self) -> MetadataProviderEnum {
		MetadataProviderEnum::Igdb
	}
	fn redis_conn(&self) -> &MultiplexedConnection {
		&self.redis
	}
	async fn match_db(self: Arc<Self>, _db_conn: &DbConn) -> anyhow::Result<()> {
		Ok(())
	}
}

fn stub_registry(redis: MultiplexedConnection) -> ProviderRegistry {
	vec![Arc::new(StubProvider { redis })]
}

async fn seed_two_providers(db: &DbConn) {
	let sql = format!(
		r#"
		INSERT INTO platform (id, name) VALUES ('{PLAT}', 'Sega CD');

		INSERT INTO signature_group (id, name, display_priority) VALUES ('{SG_NOINTRO}', 'No-Intro', 10);
		INSERT INTO signature_group (id, name, display_priority) VALUES ('{SG_REDUMP}', 'Redump', 20);

		INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id, latest_dat_file_import_id)
		VALUES ('{DF_NOINTRO}', 'No-Intro CD', '{PLAT}', '1.0', '{SG_NOINTRO}', '{IMPORT_NOINTRO}');
		INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id, latest_dat_file_import_id)
		VALUES ('{DF_REDUMP}', 'Redump CD', '{PLAT}', '1.0', '{SG_REDUMP}', '{IMPORT_REDUMP}');

		INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at)
		VALUES ('{IMPORT_NOINTRO}', '{DF_NOINTRO}', 'nointro.dat', '1.0', 'aaaaaaaa', now());
		INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at)
		VALUES ('{IMPORT_REDUMP}', '{DF_REDUMP}', 'redump.dat', '1.0', 'bbbbbbbb', now());
		"#
	);
	db.execute_unprepared(&sql).await.unwrap();
}

/// Same shape as [`seed_two_providers`] but both signature groups share one
/// display_priority, so the survivor tiebreak falls through to the lowest game
/// UUID.
async fn seed_two_providers_equal_priority(db: &DbConn) {
	let sql = format!(
		r#"
		INSERT INTO platform (id, name) VALUES ('{PLAT}', 'Sega CD');

		INSERT INTO signature_group (id, name, display_priority) VALUES ('{SG_NOINTRO}', 'No-Intro', 10);
		INSERT INTO signature_group (id, name, display_priority) VALUES ('{SG_REDUMP}', 'Redump', 10);

		INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id, latest_dat_file_import_id)
		VALUES ('{DF_NOINTRO}', 'No-Intro CD', '{PLAT}', '1.0', '{SG_NOINTRO}', '{IMPORT_NOINTRO}');
		INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id, latest_dat_file_import_id)
		VALUES ('{DF_REDUMP}', 'Redump CD', '{PLAT}', '1.0', '{SG_REDUMP}', '{IMPORT_REDUMP}');

		INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at)
		VALUES ('{IMPORT_NOINTRO}', '{DF_NOINTRO}', 'nointro.dat', '1.0', 'aaaaaaaa', now());
		INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at)
		VALUES ('{IMPORT_REDUMP}', '{DF_REDUMP}', 'redump.dat', '1.0', 'bbbbbbbb', now());
		"#
	);
	db.execute_unprepared(&sql).await.unwrap();
}

struct FileSpec {
	id: &'static str,
	name: &'static str,
	sha1: &'static str,
	sha256: Option<&'static str>,
}

async fn seed_game(db: &DbConn, game_id: &str, import_id: &str, files: &[FileSpec]) {
	let game = format!(
		r#"INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id)
		VALUES ('{game_id}', '{import_id}', 'Game {game_id}', true, '{import_id}');"#
	);
	db.execute_unprepared(&game).await.unwrap();
	for file in files {
		let sha256 = match file.sha256 {
			Some(v) => format!("'{v}'"),
			None => "NULL".to_string(),
		};
		let sql = format!(
			r#"INSERT INTO game_file (id, game_id, file_name, file_size_in_bytes, sha1, sha256, is_current, last_seen_dat_file_import_id)
			VALUES ('{}', '{game_id}', '{}', 1024, '{}', {sha256}, true, '{import_id}');"#,
			file.id, file.name, file.sha1
		);
		db.execute_unprepared(&sql).await.unwrap();
	}
}

async fn seed_mapping(
	db: &DbConn,
	mapping_id: &str,
	game_id: &str,
	provider: &str,
	provider_id: &str,
	match_type: &str,
	reason: Option<&str>,
) {
	let reason_col = match reason {
		Some(_) => ", automatic_match_reason".to_string(),
		None => String::new(),
	};
	let reason_val = match reason {
		Some(r) => format!(", '{r}'"),
		None => String::new(),
	};
	let sql = format!(
		r#"INSERT INTO signature_metadata_mapping (id, game_id, provider, provider_id, match_type, created_at, updated_at{reason_col})
		VALUES ('{mapping_id}', '{game_id}', '{provider}', '{provider_id}', '{match_type}', now(), now(){reason_val});"#
	);
	db.execute_unprepared(&sql).await.unwrap();
}

async fn anchor_for(db: &DbConn, game_id: &str) -> Option<Uuid> {
	let game = get_game_by_id(game_id.parse().unwrap(), db)
		.await
		.unwrap()
		.unwrap();
	assign_content_anchor_for_game(&game, db).await.unwrap()
}

async fn provider_id_of(
	db: &DbConn,
	game_id: &str,
	provider: MetadataProviderEnum,
) -> Option<String> {
	mapping_of(db, game_id, provider)
		.await
		.and_then(|m| m.provider_id)
}

async fn mapping_of(
	db: &DbConn,
	game_id: &str,
	provider: MetadataProviderEnum,
) -> Option<signature_metadata_mapping::Model> {
	let id: Uuid = game_id.parse().unwrap();
	signature_metadata_mapping::Entity::find()
		.all(db)
		.await
		.unwrap()
		.into_iter()
		.find(|m| m.game_id == Some(id) && m.provider == provider)
}

const GAME_A: &str = "0a0a0a0a-0a0a-0a0a-0a0a-0a0a0a0a0a0a";
const GAME_B: &str = "0b0b0b0b-0b0b-0b0b-0b0b-0b0b0b0b0b0b";

#[tokio::test]
async fn equal_full_sha1_sets_form_one_anchor() {
	let (_pg, db) = start_pg().await;
	seed_two_providers(&db).await;

	// Two multi-file games with the same two tracks (order swapped on disk).
	seed_game(
		&db,
		GAME_A,
		IMPORT_NOINTRO,
		&[
			FileSpec {
				id: "f1111111-0000-0000-0000-000000000001",
				name: "t1.bin",
				sha1: SHA1_TRACK_A,
				sha256: None,
			},
			FileSpec {
				id: "f1111111-0000-0000-0000-000000000002",
				name: "t2.bin",
				sha1: SHA1_TRACK_B,
				sha256: None,
			},
		],
	)
	.await;
	seed_game(
		&db,
		GAME_B,
		IMPORT_REDUMP,
		&[
			FileSpec {
				id: "f2222222-0000-0000-0000-000000000001",
				name: "track2.bin",
				sha1: SHA1_TRACK_B,
				sha256: None,
			},
			FileSpec {
				id: "f2222222-0000-0000-0000-000000000002",
				name: "track1.bin",
				sha1: SHA1_TRACK_A,
				sha256: None,
			},
		],
	)
	.await;

	let anchor_a = anchor_for(&db, GAME_A).await;
	let anchor_b = anchor_for(&db, GAME_B).await;

	assert!(
		anchor_a.is_some(),
		"a fully hashed multi-file game must anchor"
	);
	assert_eq!(
		anchor_a, anchor_b,
		"two games with equal full SHA1 sets must land on the same anchor"
	);

	let members = find_games_by_content_anchor(anchor_a.unwrap(), &db)
		.await
		.unwrap();
	assert_eq!(members.len(), 2, "the shared anchor must hold both games");
}

#[tokio::test]
async fn one_shared_track_does_not_merge() {
	let (_pg, db) = start_pg().await;
	seed_two_providers(&db).await;

	// Game A: tracks A and B. Game B: tracks A and C. They share only track A.
	seed_game(
		&db,
		GAME_A,
		IMPORT_NOINTRO,
		&[
			FileSpec {
				id: "f1111111-0000-0000-0000-000000000001",
				name: "t1.bin",
				sha1: SHA1_TRACK_A,
				sha256: None,
			},
			FileSpec {
				id: "f1111111-0000-0000-0000-000000000002",
				name: "t2.bin",
				sha1: SHA1_TRACK_B,
				sha256: None,
			},
		],
	)
	.await;
	seed_game(
		&db,
		GAME_B,
		IMPORT_REDUMP,
		&[
			FileSpec {
				id: "f2222222-0000-0000-0000-000000000001",
				name: "t1.bin",
				sha1: SHA1_TRACK_A,
				sha256: None,
			},
			FileSpec {
				id: "f2222222-0000-0000-0000-000000000002",
				name: "t3.bin",
				sha1: SHA1_TRACK_C,
				sha256: None,
			},
		],
	)
	.await;

	let anchor_a = anchor_for(&db, GAME_A).await;
	let anchor_b = anchor_for(&db, GAME_B).await;

	assert!(anchor_a.is_some() && anchor_b.is_some());
	assert_ne!(
		anchor_a, anchor_b,
		"a single shared track must not merge two distinct multi-file games"
	);
}

#[tokio::test]
async fn secondary_hash_veto_rejects_engineered_collision() {
	let (_pg, db) = start_pg().await;
	seed_two_providers(&db).await;

	// Identical single SHA1 but disagreeing present SHA256: an engineered
	// collision. The second game must be left anchorless.
	seed_game(
		&db,
		GAME_A,
		IMPORT_NOINTRO,
		&[FileSpec {
			id: "f1111111-0000-0000-0000-000000000001",
			name: "rom.bin",
			sha1: SHA1_TRACK_A,
			sha256: Some("1111111111111111111111111111111111111111111111111111111111111111"),
		}],
	)
	.await;
	seed_game(
		&db,
		GAME_B,
		IMPORT_REDUMP,
		&[FileSpec {
			id: "f2222222-0000-0000-0000-000000000001",
			name: "rom.bin",
			sha1: SHA1_TRACK_A,
			sha256: Some("2222222222222222222222222222222222222222222222222222222222222222"),
		}],
	)
	.await;

	let anchor_a = anchor_for(&db, GAME_A).await;
	let anchor_b = anchor_for(&db, GAME_B).await;

	assert!(anchor_a.is_some(), "the first game anchors normally");
	assert_eq!(
		anchor_b, None,
		"a disagreeing present SHA256 on the same SHA1 must veto the join and leave the game anchorless"
	);
}

#[tokio::test]
async fn find_or_create_lands_co_hashed_games_on_one_anchor() {
	let (_pg, db) = start_pg().await;
	seed_two_providers(&db).await;

	seed_game(
		&db,
		GAME_A,
		IMPORT_NOINTRO,
		&[FileSpec {
			id: "f1111111-0000-0000-0000-000000000001",
			name: "rom.bin",
			sha1: SHA1_TRACK_A,
			sha256: None,
		}],
	)
	.await;
	seed_game(
		&db,
		GAME_B,
		IMPORT_REDUMP,
		&[FileSpec {
			id: "f2222222-0000-0000-0000-000000000001",
			name: "rom.bin",
			sha1: SHA1_TRACK_A,
			sha256: None,
		}],
	)
	.await;

	let game_a = get_game_by_id(GAME_A.parse().unwrap(), &db)
		.await
		.unwrap()
		.unwrap();
	let game_b = get_game_by_id(GAME_B.parse().unwrap(), &db)
		.await
		.unwrap()
		.unwrap();

	// Race the two anchor assignments. The conflict-safe upsert must converge
	// them onto a single anchor row.
	let db_a = db.clone();
	let db_b = db.clone();
	let (a, b) = tokio::join!(
		tokio::spawn(async move {
			assign_content_anchor_for_game(&game_a, &db_a)
				.await
				.unwrap()
		}),
		tokio::spawn(async move {
			assign_content_anchor_for_game(&game_b, &db_b)
				.await
				.unwrap()
		}),
	);
	let anchor_a = a.unwrap();
	let anchor_b = b.unwrap();

	assert_eq!(
		anchor_a, anchor_b,
		"two co-hashed games racing find-or-create must land on one anchor"
	);
	let members = find_games_by_content_anchor(anchor_a.unwrap(), &db)
		.await
		.unwrap();
	assert_eq!(
		members.len(),
		2,
		"both racing games must share the one anchor"
	);
}

#[tokio::test]
async fn reconcile_prefers_manual_and_never_downgrades_it() {
	let (_pg, db) = start_pg().await;
	let (_redis, redis) = start_redis().await;
	seed_two_providers(&db).await;

	seed_game(
		&db,
		GAME_A,
		IMPORT_NOINTRO,
		&[FileSpec {
			id: "f1111111-0000-0000-0000-000000000001",
			name: "rom.bin",
			sha1: SHA1_TRACK_A,
			sha256: None,
		}],
	)
	.await;
	seed_game(
		&db,
		GAME_B,
		IMPORT_REDUMP,
		&[FileSpec {
			id: "f2222222-0000-0000-0000-000000000001",
			name: "rom.bin",
			sha1: SHA1_TRACK_A,
			sha256: None,
		}],
	)
	.await;

	// Game A (lower display_priority) carries an automatic IGDB id; game B
	// carries a Manual IGDB id with a different provider id. The Manual must win
	// and must not be downgraded.
	seed_mapping(
		&db,
		"d0000000-0000-0000-0000-000000000001",
		GAME_A,
		"igdb",
		"auto-100",
		"automatic",
		Some("direct_name"),
	)
	.await;
	seed_mapping(
		&db,
		"d0000000-0000-0000-0000-000000000002",
		GAME_B,
		"igdb",
		"manual-200",
		"manual",
		None,
	)
	.await;

	anchor_for(&db, GAME_A).await.unwrap();
	anchor_for(&db, GAME_B).await.unwrap();

	let registry = stub_registry(redis);
	run_content_anchor_reconcile_wave(&registry, &db)
		.await
		.unwrap();

	assert_eq!(
		provider_id_of(&db, GAME_B, MetadataProviderEnum::Igdb)
			.await
			.as_deref(),
		Some("manual-200"),
		"a Manual mapping must never be downgraded by reconcile"
	);
	let propagated = mapping_of(&db, GAME_A, MetadataProviderEnum::Igdb)
		.await
		.unwrap();
	assert_eq!(
		propagated.provider_id.as_deref(),
		Some("manual-200"),
		"the automatic member must converge to the Manual survivor's id"
	);
	assert_eq!(
		propagated.match_type,
		MatchTypeEnum::Automatic,
		"a content-hash propagation writes an Automatic mapping on the receiving member, never a Manual"
	);
	assert_eq!(
		propagated.automatic_match_reason,
		Some(AutomaticMatchReasonEnum::ViaContentHash),
		"the propagated mapping must record the content-hash reason"
	);
}

#[tokio::test]
async fn reconcile_leaves_both_manual_disagreements_untouched() {
	let (_pg, db) = start_pg().await;
	let (_redis, redis) = start_redis().await;
	seed_two_providers(&db).await;

	seed_game(
		&db,
		GAME_A,
		IMPORT_NOINTRO,
		&[FileSpec {
			id: "f1111111-0000-0000-0000-000000000001",
			name: "rom.bin",
			sha1: SHA1_TRACK_A,
			sha256: None,
		}],
	)
	.await;
	seed_game(
		&db,
		GAME_B,
		IMPORT_REDUMP,
		&[FileSpec {
			id: "f2222222-0000-0000-0000-000000000001",
			name: "rom.bin",
			sha1: SHA1_TRACK_A,
			sha256: None,
		}],
	)
	.await;

	seed_mapping(
		&db,
		"d0000000-0000-0000-0000-000000000001",
		GAME_A,
		"igdb",
		"manual-aaa",
		"manual",
		None,
	)
	.await;
	seed_mapping(
		&db,
		"d0000000-0000-0000-0000-000000000002",
		GAME_B,
		"igdb",
		"manual-bbb",
		"manual",
		None,
	)
	.await;

	anchor_for(&db, GAME_A).await.unwrap();
	anchor_for(&db, GAME_B).await.unwrap();

	let registry = stub_registry(redis);
	run_content_anchor_reconcile_wave(&registry, &db)
		.await
		.unwrap();

	assert_eq!(
		provider_id_of(&db, GAME_A, MetadataProviderEnum::Igdb)
			.await
			.as_deref(),
		Some("manual-aaa"),
		"a both-Manual disagreement must leave game A untouched"
	);
	assert_eq!(
		provider_id_of(&db, GAME_B, MetadataProviderEnum::Igdb)
			.await
			.as_deref(),
		Some("manual-bbb"),
		"a both-Manual disagreement must leave game B untouched"
	);
}

#[tokio::test]
async fn reconcile_is_idempotent_on_a_converged_class() {
	let (_pg, db) = start_pg().await;
	let (_redis, redis) = start_redis().await;
	seed_two_providers(&db).await;

	seed_game(
		&db,
		GAME_A,
		IMPORT_NOINTRO,
		&[FileSpec {
			id: "f1111111-0000-0000-0000-000000000001",
			name: "rom.bin",
			sha1: SHA1_TRACK_A,
			sha256: None,
		}],
	)
	.await;
	seed_game(
		&db,
		GAME_B,
		IMPORT_REDUMP,
		&[FileSpec {
			id: "f2222222-0000-0000-0000-000000000001",
			name: "rom.bin",
			sha1: SHA1_TRACK_A,
			sha256: None,
		}],
	)
	.await;

	seed_mapping(
		&db,
		"d0000000-0000-0000-0000-000000000001",
		GAME_A,
		"igdb",
		"auto-100",
		"automatic",
		Some("direct_name"),
	)
	.await;

	anchor_for(&db, GAME_A).await.unwrap();
	anchor_for(&db, GAME_B).await.unwrap();

	let registry = stub_registry(redis);

	run_content_anchor_reconcile_wave(&registry, &db)
		.await
		.unwrap();
	let after_first = mapping_of(&db, GAME_B, MetadataProviderEnum::Igdb)
		.await
		.unwrap();
	assert_eq!(after_first.provider_id.as_deref(), Some("auto-100"));

	// A converged class must write nothing on the next pass: the propagated
	// mapping's updated_at must be unchanged.
	tokio::time::sleep(std::time::Duration::from_millis(50)).await;
	run_content_anchor_reconcile_wave(&registry, &db)
		.await
		.unwrap();
	let after_second = mapping_of(&db, GAME_B, MetadataProviderEnum::Igdb)
		.await
		.unwrap();

	assert_eq!(
		after_first.updated_at, after_second.updated_at,
		"a converged anchor must write nothing on a second reconcile pass"
	);
}

#[tokio::test]
async fn single_member_anchor_is_not_visited_by_reconcile() {
	let (_pg, db) = start_pg().await;
	seed_two_providers(&db).await;

	seed_game(
		&db,
		GAME_A,
		IMPORT_NOINTRO,
		&[FileSpec {
			id: "f1111111-0000-0000-0000-000000000001",
			name: "rom.bin",
			sha1: SHA1_TRACK_A,
			sha256: None,
		}],
	)
	.await;

	anchor_for(&db, GAME_A).await.unwrap();

	let multi = find_multi_member_anchor_ids(&db).await.unwrap();
	assert!(
		multi.is_empty(),
		"a lone anchored game must not appear in the multi-member set the reconcile wave visits"
	);
}

#[tokio::test]
async fn one_present_one_null_sha256_still_co_anchors_on_shared_sha1() {
	let (_pg, db) = start_pg().await;
	seed_two_providers(&db).await;

	// Same single SHA1. One side carries a SHA256, the other leaves it NULL. With
	// only one secondary hash present there is nothing to disagree on, so the veto
	// does not fire and both games land on the shared anchor.
	seed_game(
		&db,
		GAME_A,
		IMPORT_NOINTRO,
		&[FileSpec {
			id: "f1111111-0000-0000-0000-000000000001",
			name: "rom.bin",
			sha1: SHA1_TRACK_A,
			sha256: Some("1111111111111111111111111111111111111111111111111111111111111111"),
		}],
	)
	.await;
	seed_game(
		&db,
		GAME_B,
		IMPORT_REDUMP,
		&[FileSpec {
			id: "f2222222-0000-0000-0000-000000000001",
			name: "rom.bin",
			sha1: SHA1_TRACK_A,
			sha256: None,
		}],
	)
	.await;

	let anchor_a = anchor_for(&db, GAME_A).await;
	let anchor_b = anchor_for(&db, GAME_B).await;

	assert!(anchor_a.is_some(), "the fully hashed game anchors normally");
	assert_eq!(
		anchor_a, anchor_b,
		"a present-vs-NULL SHA256 pair on one shared SHA1 must co-anchor, not veto"
	);
	let members = find_games_by_content_anchor(anchor_a.unwrap(), &db)
		.await
		.unwrap();
	assert_eq!(members.len(), 2, "both games must share the one anchor");
}

#[tokio::test]
async fn equal_display_priority_breaks_the_survivor_tie_by_lowest_game_id() {
	let (_pg, db) = start_pg().await;
	let (_redis, redis) = start_redis().await;
	seed_two_providers_equal_priority(&db).await;

	seed_game(
		&db,
		GAME_A,
		IMPORT_NOINTRO,
		&[FileSpec {
			id: "f1111111-0000-0000-0000-000000000001",
			name: "rom.bin",
			sha1: SHA1_TRACK_A,
			sha256: None,
		}],
	)
	.await;
	seed_game(
		&db,
		GAME_B,
		IMPORT_REDUMP,
		&[FileSpec {
			id: "f2222222-0000-0000-0000-000000000001",
			name: "rom.bin",
			sha1: SHA1_TRACK_A,
			sha256: None,
		}],
	)
	.await;

	// Both members carry an automatic IGDB id of equal strength under equal
	// display_priority, so the survivor is decided by the lower game UUID. GAME_A
	// sorts below GAME_B, so its id must win and propagate onto GAME_B.
	seed_mapping(
		&db,
		"d0000000-0000-0000-0000-000000000001",
		GAME_A,
		"igdb",
		"auto-aaa",
		"automatic",
		Some("direct_name"),
	)
	.await;
	seed_mapping(
		&db,
		"d0000000-0000-0000-0000-000000000002",
		GAME_B,
		"igdb",
		"auto-bbb",
		"automatic",
		Some("direct_name"),
	)
	.await;

	anchor_for(&db, GAME_A).await.unwrap();
	anchor_for(&db, GAME_B).await.unwrap();

	let registry = stub_registry(redis);
	run_content_anchor_reconcile_wave(&registry, &db)
		.await
		.unwrap();

	assert_eq!(
		provider_id_of(&db, GAME_B, MetadataProviderEnum::Igdb)
			.await
			.as_deref(),
		Some("auto-aaa"),
		"under an equal-priority tie the lowest game UUID wins and propagates onto the sibling"
	);
	assert_eq!(
		provider_id_of(&db, GAME_A, MetadataProviderEnum::Igdb)
			.await
			.as_deref(),
		Some("auto-aaa"),
		"the survivor's own mapping is left intact"
	);
}
