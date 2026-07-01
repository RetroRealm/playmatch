use sea_orm_migration::prelude::*;

/// Backs the v2 `(name, id)` keyset browse over game. The existing trigram GIN
/// index serves fuzzy search but cannot drive an ordered range scan, so add a
/// plain btree on (name, id).
#[derive(Iden)]
enum Game {
	Table,
	Name,
	Id,
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		// Plain CREATE INDEX, not CONCURRENTLY: the migration framework runs each
		// step in a transaction and CONCURRENTLY cannot run there. On a large game
		// table this takes a brief write lock, so deploy it off-peak.
		manager
			.create_index(
				Index::create()
					.if_not_exists()
					.name("idx_game_name_id")
					.table(Game::Table)
					.col(Game::Name)
					.col(Game::Id)
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.drop_index(
				Index::drop()
					.if_exists()
					.name("idx_game_name_id")
					.table(Game::Table)
					.to_owned(),
			)
			.await
	}
}
