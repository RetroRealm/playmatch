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
				"ALTER TABLE signature_metadata_mapping \
				 ADD COLUMN IF NOT EXISTS matched_year SMALLINT NULL;",
			)
			.await?;
		manager
			.get_connection()
			.execute_unprepared(
				"ALTER TYPE failed_match_reason_enum ADD VALUE IF NOT EXISTS 'ambiguous';",
			)
			.await?;
		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.get_connection()
			.execute_unprepared(
				"ALTER TABLE signature_metadata_mapping DROP COLUMN IF EXISTS matched_year;",
			)
			.await?;
		Ok(())
	}
}
