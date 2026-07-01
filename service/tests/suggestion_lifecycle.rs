//! Postgres-backed tests for the suggestion accept/decline lifecycle.
//! Require Docker.

use migration::{Migrator, MigratorTrait};
use redis::aio::MultiplexedConnection;
use sea_orm::prelude::Uuid;
use sea_orm::{ConnectionTrait, Database, DbConn, Statement};
use service::matching::suggestions::{
	accept_suggestion, add_game_suggestion, decline_suggestion, get_suggestions_page,
};
use service::model::MetadataProvider;
use service::model::suggestion::GameSuggestionRequest;
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
const GAME_NAME: &str = "Pokemon - Diamant-Edition (Germany) (Rev 5)";
const PROVIDER_ID: &str = "7338";

async fn start_pg() -> (ContainerAsync<Postgres>, DbConn) {
	// Matches the production major version.
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

async fn seed_game(db: &DbConn) {
	let sql = format!(
		r#"
		INSERT INTO signature_group (id, name) VALUES ('{SG}', 'No-Intro');
		INSERT INTO platform (id, name) VALUES ('{PLAT}', 'Nintendo DS');

		INSERT INTO dat_file (id, name, platform_id, current_version, signature_group_id, latest_dat_file_import_id)
		VALUES ('{DF}', 'Nintendo - Nintendo DS (Decrypted)', '{PLAT}', '20260617-122122', '{SG}', '{IMPORT}');

		INSERT INTO dat_file_import (id, dat_file_id, name, version, md5, imported_at)
		VALUES ('{IMPORT}', '{DF}', 'Nintendo - Nintendo DS (Decrypted) (20260617-122122).dat', '20260617-122122', 'aaaaaaaa', '2026-06-18 12:00:00+00');

		INSERT INTO game (id, dat_file_import_id, name, is_current, last_seen_dat_file_import_id)
		VALUES ('{GAME}', '{IMPORT}', '{GAME_NAME}', true, '{IMPORT}');

		INSERT INTO game_file (id, game_id, file_name, sha1, is_current, last_seen_dat_file_import_id)
		VALUES ('{GF}', '{GAME}', '{GAME_NAME}.nds', '{SHA1}', true, '{IMPORT}');
		"#
	);
	db.execute_unprepared(&sql).await.unwrap();
}

fn game_request() -> GameSuggestionRequest {
	GameSuggestionRequest {
		md5: None,
		sha1: Some(SHA1.to_string()),
		sha256: None,
		name: None,
		comment: Some("looks like IGDB 7338".to_string()),
		provider: MetadataProvider::IGDB,
		provider_id: PROVIDER_ID.to_string(),
		user_id: None,
	}
}

async fn count_suggestions(db: &DbConn) -> i64 {
	scalar_count(
		db,
		"SELECT count(*) AS cnt FROM signature_metadata_mapping_suggestions",
	)
	.await
}

async fn scalar_count(db: &DbConn, sql: &str) -> i64 {
	let row = db
		.query_one(Statement::from_string(
			db.get_database_backend(),
			sql.to_owned(),
		))
		.await
		.unwrap()
		.unwrap();
	row.try_get::<i64>("", "cnt").unwrap()
}

#[tokio::test]
async fn accept_game_suggestion_creates_mapping_and_clears_suggestion() {
	let (_pg, db) = start_pg().await;
	let (_redis, mut redis) = start_redis().await;
	seed_game(&db).await;

	let suggestion = add_game_suggestion(game_request(), &db).await.unwrap();
	assert_eq!(
		suggestion.game_id,
		Some(Uuid::parse_str(GAME).unwrap()),
		"the new suggestion must target the resolved game"
	);
	assert_eq!(
		count_suggestions(&db).await,
		1,
		"the suggestion must persist"
	);

	let updated = accept_suggestion(suggestion.id, &db, &mut redis)
		.await
		.unwrap();
	assert!(
		updated >= 1,
		"accepting must report at least one updated mapping, got {updated}"
	);

	let row = db
		.query_one(Statement::from_string(
			db.get_database_backend(),
			format!(
				r#"
				SELECT provider_id, provider::text AS provider, match_type::text AS match_type,
				       manual_match_type::text AS manual_match_type
				FROM signature_metadata_mapping
				WHERE game_id = '{GAME}'
				"#
			),
		))
		.await
		.unwrap()
		.expect("accepting must create a signature_metadata_mapping for the game");

	assert_eq!(
		row.try_get::<String>("", "provider_id").unwrap(),
		PROVIDER_ID,
		"the accepted mapping must carry the suggestion's provider id"
	);
	assert_eq!(
		row.try_get::<String>("", "provider").unwrap(),
		"igdb",
		"the accepted mapping must carry the suggestion's provider"
	);
	assert_eq!(
		row.try_get::<String>("", "match_type").unwrap(),
		"manual",
		"an accepted suggestion lands as a manual match"
	);
	assert_eq!(
		row.try_get::<String>("", "manual_match_type").unwrap(),
		"community",
		"an accepted suggestion is a community manual match"
	);

	// The lifecycle marks an accepted suggestion terminal by deleting the row;
	// there is no status column, so the row is simply gone.
	assert_eq!(
		count_suggestions(&db).await,
		0,
		"an accepted suggestion must be removed from the open suggestions"
	);
}

#[tokio::test]
async fn decline_suggestion_creates_no_mapping_and_clears_suggestion() {
	let (_pg, db) = start_pg().await;
	seed_game(&db).await;

	let suggestion = add_game_suggestion(game_request(), &db).await.unwrap();
	assert_eq!(
		count_suggestions(&db).await,
		1,
		"the suggestion must persist"
	);

	decline_suggestion(suggestion.id, &db).await.unwrap();

	assert_eq!(
		scalar_count(
			&db,
			"SELECT count(*) AS cnt FROM signature_metadata_mapping",
		)
		.await,
		0,
		"declining must not create any signature_metadata_mapping"
	);
	assert_eq!(
		count_suggestions(&db).await,
		0,
		"a declined suggestion must be removed from the open suggestions"
	);
}

#[tokio::test]
async fn get_suggestions_page_returns_keyset_order_newest_first() {
	let (_pg, db) = start_pg().await;
	seed_game(&db).await;

	let oldest = "10101010-1010-1010-1010-101010101010";
	let middle = "20202020-2020-2020-2020-202020202020";
	let newest = "30303030-3030-3030-3030-303030303030";

	let sql = format!(
		r#"
		INSERT INTO signature_metadata_mapping_suggestions (id, game_id, provider, provider_id, created_at, updated_at)
		VALUES ('{oldest}', '{GAME}', 'igdb', 'p-oldest', '2026-01-01 00:00:00+00', '2026-01-01 00:00:00+00');
		INSERT INTO signature_metadata_mapping_suggestions (id, game_id, provider, provider_id, created_at, updated_at)
		VALUES ('{middle}', '{GAME}', 'igdb', 'p-middle', '2026-02-01 00:00:00+00', '2026-02-01 00:00:00+00');
		INSERT INTO signature_metadata_mapping_suggestions (id, game_id, provider, provider_id, created_at, updated_at)
		VALUES ('{newest}', '{GAME}', 'igdb', 'p-newest', '2026-03-01 00:00:00+00', '2026-03-01 00:00:00+00');
		"#
	);
	db.execute_unprepared(&sql).await.unwrap();

	let first = get_suggestions_page(None, Some(2), &db).await.unwrap();
	let first_ids: Vec<String> = first.rows.iter().map(|s| s.id.to_string()).collect();
	assert_eq!(
		first_ids,
		vec![newest.to_string(), middle.to_string()],
		"the first page must hold the two newest suggestions, newest first"
	);
	assert!(
		first.has_more,
		"with three rows and a limit of two there must be a further page"
	);

	let last = first.rows.last().unwrap();
	let cursor = (last.created_at, last.id);
	let second = get_suggestions_page(Some(cursor), Some(2), &db)
		.await
		.unwrap();
	let second_ids: Vec<String> = second.rows.iter().map(|s| s.id.to_string()).collect();
	assert_eq!(
		second_ids,
		vec![oldest.to_string()],
		"the next page must continue strictly older than the cursor"
	);
	assert!(
		!second.has_more,
		"the oldest row exhausts the keyset, so no further page"
	);
}
