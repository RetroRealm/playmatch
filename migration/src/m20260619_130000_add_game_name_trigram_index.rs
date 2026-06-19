use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let conn = manager.get_connection();

		conn.execute_unprepared(r#"CREATE EXTENSION IF NOT EXISTS pg_trgm;"#)
			.await?;

		// Plain CREATE INDEX, not CONCURRENTLY: the migration framework runs each
		// step in a transaction and CONCURRENTLY cannot run there. On a large game
		// table this takes a brief write lock, so deploy it off-peak.
		conn.execute_unprepared(
			r#"
            CREATE INDEX IF NOT EXISTS idx_game_name_trgm
              ON game USING gin (name gin_trgm_ops);
            "#,
		)
		.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.get_connection()
			.execute_unprepared(r#"DROP INDEX IF EXISTS idx_game_name_trgm;"#)
			.await?;

		Ok(())
	}
}
