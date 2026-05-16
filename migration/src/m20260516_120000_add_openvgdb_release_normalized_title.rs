use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

// Existing rows leave `title_name_normalized` NULL until the next OpenVGDB
// import runs. To force an immediate refresh:
//   DELETE FROM openvgdb_import;
// then restart playmatch (or wait for the noon UTC cron). The matcher's
// normalized rung skips rows where the column is NULL.
#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let conn = manager.get_connection();

		conn.execute_unprepared(
			"ALTER TABLE openvgdb_release \
			 ADD COLUMN IF NOT EXISTS title_name_normalized TEXT NULL;",
		)
		.await?;

		conn.execute_unprepared(
			"CREATE INDEX IF NOT EXISTS idx_openvgdb_release_title_lower \
			 ON openvgdb_release (lower(title_name));",
		)
		.await?;

		conn.execute_unprepared(
			"CREATE INDEX IF NOT EXISTS idx_openvgdb_release_title_normalized \
			 ON openvgdb_release (lower(title_name_normalized));",
		)
		.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let conn = manager.get_connection();

		conn.execute_unprepared("DROP INDEX IF EXISTS idx_openvgdb_release_title_normalized;")
			.await?;

		conn.execute_unprepared("DROP INDEX IF EXISTS idx_openvgdb_release_title_lower;")
			.await?;

		conn.execute_unprepared(
			"ALTER TABLE openvgdb_release DROP COLUMN IF EXISTS title_name_normalized;",
		)
		.await?;

		Ok(())
	}
}
