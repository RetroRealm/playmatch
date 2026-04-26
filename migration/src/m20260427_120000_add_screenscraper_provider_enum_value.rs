use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.get_connection()
			.execute_unprepared(
				"ALTER TYPE metadata_provider_enum ADD VALUE IF NOT EXISTS 'screenscraper';",
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
