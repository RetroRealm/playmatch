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
				"ALTER TYPE failed_match_reason_enum ADD VALUE IF NOT EXISTS 'too_many_files';",
			)
			.await?;
		Ok(())
	}

	// Down is a no-op: enum-extension migrations cannot be reverted, see the crate doc in lib.rs.
	async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
		Ok(())
	}
}
