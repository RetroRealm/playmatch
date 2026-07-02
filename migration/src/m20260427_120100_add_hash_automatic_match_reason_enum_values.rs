use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let conn = manager.get_connection();
		conn.execute_unprepared(
			"ALTER TYPE automatic_match_reason_enum ADD VALUE IF NOT EXISTS 'md5_hash';",
		)
		.await?;
		conn.execute_unprepared(
			"ALTER TYPE automatic_match_reason_enum ADD VALUE IF NOT EXISTS 'sha1_hash';",
		)
		.await?;
		conn.execute_unprepared(
			"ALTER TYPE automatic_match_reason_enum ADD VALUE IF NOT EXISTS 'crc_hash';",
		)
		.await?;
		Ok(())
	}

	// Down is a no-op: enum-extension migrations cannot be reverted, see the crate doc in lib.rs.
	async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
		Ok(())
	}
}
