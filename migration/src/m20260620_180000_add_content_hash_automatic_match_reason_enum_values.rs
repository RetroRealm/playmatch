use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let conn = manager.get_connection();
		conn.execute_unprepared(
			"ALTER TYPE automatic_match_reason_enum ADD VALUE IF NOT EXISTS 'via_content_hash';",
		)
		.await?;
		conn.execute_unprepared(
			"ALTER TYPE automatic_match_reason_enum ADD VALUE IF NOT EXISTS 'sha256_hash';",
		)
		.await?;
		Ok(())
	}

	// Postgres has no DROP VALUE for enum types without rewriting every column
	// that references it; reverting an enum-extension migration is intentionally
	// a no-op. To roll back, drop the dependent rows and recreate the type.
	async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
		Ok(())
	}
}
