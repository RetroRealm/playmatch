use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let conn = manager.get_connection();

		// The unique index makes presence inserts idempotent, so re-running an
		// import never duplicates an observation.
		conn.execute_unprepared(
			r#"
			CREATE TABLE IF NOT EXISTS game_file_presence (
				id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
				game_file_id uuid NOT NULL REFERENCES game_file(id) ON DELETE CASCADE,
				dat_file_import_id uuid NOT NULL REFERENCES dat_file_import(id) ON DELETE CASCADE,
				created_at timestamptz NOT NULL DEFAULT now()
			);
			CREATE UNIQUE INDEX IF NOT EXISTS idx_game_file_presence_unique
				ON game_file_presence (game_file_id, dat_file_import_id);
			CREATE INDEX IF NOT EXISTS idx_game_file_presence_import
				ON game_file_presence (dat_file_import_id);
			"#,
		)
		.await?;

		// Denormalized so the identify path never aggregates the presence table.
		// is_current defaults to true because every existing row was present at
		// its last import; the post-deploy re-import then sets the exact state.
		// No FK constraint, so these column adds stay metadata-only on a large table.
		conn.execute_unprepared(
			r#"
			ALTER TABLE game_file ADD COLUMN IF NOT EXISTS last_seen_dat_file_import_id uuid;
			ALTER TABLE game_file ADD COLUMN IF NOT EXISTS is_current boolean NOT NULL DEFAULT true;
			ALTER TABLE game ADD COLUMN IF NOT EXISTS last_seen_dat_file_import_id uuid;
			ALTER TABLE game ADD COLUMN IF NOT EXISTS is_current boolean NOT NULL DEFAULT true;
			ALTER TABLE dat_file ADD COLUMN IF NOT EXISTS latest_dat_file_import_id uuid;
			"#,
		)
		.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let conn = manager.get_connection();

		conn.execute_unprepared(
			r#"
			ALTER TABLE game_file DROP COLUMN IF EXISTS last_seen_dat_file_import_id;
			ALTER TABLE game_file DROP COLUMN IF EXISTS is_current;
			ALTER TABLE game DROP COLUMN IF EXISTS last_seen_dat_file_import_id;
			ALTER TABLE game DROP COLUMN IF EXISTS is_current;
			ALTER TABLE dat_file DROP COLUMN IF EXISTS latest_dat_file_import_id;
			DROP TABLE IF EXISTS game_file_presence;
			"#,
		)
		.await?;

		Ok(())
	}
}
