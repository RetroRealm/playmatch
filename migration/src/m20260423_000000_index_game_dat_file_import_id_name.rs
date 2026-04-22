use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let conn = manager.get_connection();

		// Compound index backing find_game_by_name_and_dat_file_id, called in
		// a per-game inner loop during DAT ingestion. Without this, re-imports
		// degrade to O(n^2) as the game table grows.
		conn.execute_unprepared(
			r#"
                CREATE INDEX IF NOT EXISTS idx_game_dat_file_import_id_name
                  ON game (dat_file_import_id, name);
                "#,
		)
		.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.get_connection()
			.execute_unprepared(r#"DROP INDEX IF EXISTS idx_game_dat_file_import_id_name;"#)
			.await?;

		Ok(())
	}
}
