use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let conn = manager.get_connection();

		conn.execute_unprepared(
			"ALTER TABLE launchbox_game \
			 ADD COLUMN IF NOT EXISTS name_normalized TEXT NULL;",
		)
		.await?;

		conn.execute_unprepared(
			"ALTER TABLE launchbox_game_alternate_name \
			 ADD COLUMN IF NOT EXISTS name_normalized TEXT NULL;",
		)
		.await?;

		conn.execute_unprepared(
			"CREATE INDEX IF NOT EXISTS idx_launchbox_game_platform_name_normalized_lower \
			 ON launchbox_game (lower(platform_name), lower(name_normalized));",
		)
		.await?;

		conn.execute_unprepared(
			"CREATE INDEX IF NOT EXISTS idx_launchbox_game_alternate_name_normalized_lower \
			 ON launchbox_game_alternate_name (lower(name_normalized));",
		)
		.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let conn = manager.get_connection();

		conn.execute_unprepared(
			"DROP INDEX IF EXISTS idx_launchbox_game_alternate_name_normalized_lower;",
		)
		.await?;

		conn.execute_unprepared(
			"DROP INDEX IF EXISTS idx_launchbox_game_platform_name_normalized_lower;",
		)
		.await?;

		conn.execute_unprepared(
			"ALTER TABLE launchbox_game_alternate_name DROP COLUMN IF EXISTS name_normalized;",
		)
		.await?;

		conn.execute_unprepared(
			"ALTER TABLE launchbox_game DROP COLUMN IF EXISTS name_normalized;",
		)
		.await?;

		Ok(())
	}
}
