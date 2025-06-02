use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.get_connection()
			.execute_unprepared(
				r#"
                CREATE INDEX IF NOT EXISTS idx_game_file_lower_md5
                  ON game_file ((LOWER("md5")));
                "#,
			)
			.await?;

		manager
			.get_connection()
			.execute_unprepared(
				r#"
                CREATE INDEX IF NOT EXISTS idx_game_file_lower_sha1
                  ON game_file ((LOWER("sha1")));
                "#,
			)
			.await?;

		manager
			.get_connection()
			.execute_unprepared(
				r#"
                CREATE INDEX IF NOT EXISTS idx_game_file_lower_sha256
                  ON game_file ((LOWER("sha256")));
                "#,
			)
			.await?;

		manager
			.get_connection()
			.execute_unprepared(
				r#"
                CREATE INDEX IF NOT EXISTS idx_game_file_lower_crc
                  ON game_file ((LOWER("crc")));
                "#,
			)
			.await?;

		manager
			.get_connection()
			.execute_unprepared(
				r#"
                CREATE INDEX IF NOT EXISTS idx_game_file_lower_file_name
                  ON game_file ((LOWER("file_name")));
                "#,
			)
			.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.get_connection()
			.execute_unprepared(
				r#"
                DROP INDEX IF EXISTS idx_game_file_lower_md5;
                "#,
			)
			.await?;

		// Index auf LOWER("sha1") entfernen
		manager
			.get_connection()
			.execute_unprepared(
				r#"
                DROP INDEX IF EXISTS idx_game_file_lower_sha1;
                "#,
			)
			.await?;

		manager
			.get_connection()
			.execute_unprepared(
				r#"
                DROP INDEX IF EXISTS idx_game_file_lower_sha256;
                "#,
			)
			.await?;

		manager
			.get_connection()
			.execute_unprepared(
				r#"
                DROP INDEX IF EXISTS idx_game_file_lower_crc;
                "#,
			)
			.await?;

		manager
			.get_connection()
			.execute_unprepared(
				r#"
                DROP INDEX IF EXISTS idx_game_file_lower_file_name;
                "#,
			)
			.await?;

		Ok(())
	}
}
