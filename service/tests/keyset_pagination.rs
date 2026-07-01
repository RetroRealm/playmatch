//! Postgres-backed tests for the keyset pagination helper. Require Docker.

use migration::{Migrator, MigratorTrait};
use sea_orm::prelude::Uuid;
use sea_orm::{ActiveModelTrait, ColumnTrait, Database, DbConn, EntityTrait, QueryFilter, Set};
use service::db::pagination::fetch_keyset_page;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use testcontainers_modules::testcontainers::{ContainerAsync, ImageExt};

const PREFIX: &str = "zzz-keyset-test-";

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

/// Seed `count` signature groups named `<PREFIX><nnn>` so the ordered set is
/// deterministic and isolated from the rows the migrations already insert.
async fn seed_groups(db: &DbConn, count: usize) {
	for i in 0..count {
		entity::signature_group::ActiveModel {
			id: Set(Uuid::new_v4()),
			name: Set(format!("{PREFIX}{i:03}")),
			website_link: Set(None),
			description: Set(None),
			..Default::default()
		}
		.insert(db)
		.await
		.unwrap();
	}
}

#[tokio::test]
async fn has_more_true_when_overflow_row_exists() {
	let (_pg, db) = start_pg().await;
	seed_groups(&db, 5).await;

	let mut cursor = entity::signature_group::Entity::find()
		.filter(entity::signature_group::Column::Name.starts_with(PREFIX))
		.cursor_by((
			entity::signature_group::Column::Name,
			entity::signature_group::Column::Id,
		));

	let page = fetch_keyset_page(&mut cursor, Some(3), &db).await.unwrap();
	assert_eq!(page.rows.len(), 3, "page must be capped at the limit");
	assert!(page.has_more, "two rows remain after this page");
	assert_eq!(page.rows[0].name, format!("{PREFIX}000"));
	assert_eq!(page.rows[2].name, format!("{PREFIX}002"));
}

#[tokio::test]
async fn has_more_false_on_exact_and_final_page() {
	let (_pg, db) = start_pg().await;
	seed_groups(&db, 3).await;

	let mut cursor = entity::signature_group::Entity::find()
		.filter(entity::signature_group::Column::Name.starts_with(PREFIX))
		.cursor_by((
			entity::signature_group::Column::Name,
			entity::signature_group::Column::Id,
		));

	let page = fetch_keyset_page(&mut cursor, Some(3), &db).await.unwrap();
	assert_eq!(page.rows.len(), 3);
	assert!(
		!page.has_more,
		"a page that exactly drains the set has nothing after it"
	);
}

#[tokio::test]
async fn after_seeks_past_the_previous_page() {
	let (_pg, db) = start_pg().await;
	seed_groups(&db, 5).await;

	let base = || {
		entity::signature_group::Entity::find()
			.filter(entity::signature_group::Column::Name.starts_with(PREFIX))
			.cursor_by((
				entity::signature_group::Column::Name,
				entity::signature_group::Column::Id,
			))
	};

	let mut first = base();
	let first_page = fetch_keyset_page(&mut first, Some(2), &db).await.unwrap();
	assert!(first_page.has_more);
	let last = first_page.rows.last().unwrap();

	let mut second = base();
	second.after((last.name.clone(), last.id));
	let second_page = fetch_keyset_page(&mut second, Some(2), &db).await.unwrap();

	assert_eq!(second_page.rows[0].name, format!("{PREFIX}002"));
	assert_eq!(second_page.rows[1].name, format!("{PREFIX}003"));
	assert!(second_page.has_more, "one row still trails the second page");
}

#[tokio::test]
async fn limit_is_clamped_to_max() {
	let (_pg, db) = start_pg().await;
	seed_groups(&db, 55).await;

	let mut cursor = entity::signature_group::Entity::find()
		.filter(entity::signature_group::Column::Name.starts_with(PREFIX))
		.cursor_by((
			entity::signature_group::Column::Name,
			entity::signature_group::Column::Id,
		));

	let page = fetch_keyset_page(&mut cursor, Some(1000), &db)
		.await
		.unwrap();
	assert_eq!(page.rows.len(), 50, "limit above the cap is clamped to 50");
	assert!(page.has_more, "five rows remain beyond the clamped page");
}
