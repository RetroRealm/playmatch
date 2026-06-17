use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let conn = manager.get_connection();

		// Built in-transaction, not CONCURRENTLY: a concurrent build waits on the
		// migration's own transaction and deadlocks. Drop first so an INVALID
		// index left by an aborted build is rebuilt, not skipped by IF NOT EXISTS.
		conn.execute_unprepared(
			r#"
                DROP INDEX IF EXISTS idx_game_file_retired;
                CREATE INDEX IF NOT EXISTS idx_game_file_retired
                  ON game_file (game_id) WHERE is_current = false;
                "#,
		)
		.await?;

		conn.execute_unprepared(
			r#"
                DROP INDEX IF EXISTS idx_game_file_last_seen_import;
                CREATE INDEX IF NOT EXISTS idx_game_file_last_seen_import
                  ON game_file (last_seen_dat_file_import_id)
                  WHERE last_seen_dat_file_import_id IS NOT NULL;
                "#,
		)
		.await?;

		conn.execute_unprepared(
			r#"
                DROP INDEX IF EXISTS idx_game_last_seen_import;
                CREATE INDEX IF NOT EXISTS idx_game_last_seen_import
                  ON game (last_seen_dat_file_import_id)
                  WHERE last_seen_dat_file_import_id IS NOT NULL;
                "#,
		)
		.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let conn = manager.get_connection();

		conn.execute_unprepared(r#"DROP INDEX IF EXISTS idx_game_file_retired;"#)
			.await?;
		conn.execute_unprepared(r#"DROP INDEX IF EXISTS idx_game_file_last_seen_import;"#)
			.await?;
		conn.execute_unprepared(r#"DROP INDEX IF EXISTS idx_game_last_seen_import;"#)
			.await?;

		Ok(())
	}
}
