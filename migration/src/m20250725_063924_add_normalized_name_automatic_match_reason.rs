use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.get_connection()
			.execute_unprepared(
				r#"ALTER TYPE automatic_match_reason_enum ADD VALUE IF NOT EXISTS 'normalized_name';"#,
			)
			.await?;

		manager
			.get_connection()
			.execute_unprepared(
				r#"ALTER TYPE automatic_match_reason_enum ADD VALUE IF NOT EXISTS 'normalized_alternative_name';"#,
			)
			.await?;

		Ok(())
	}

	async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
		// Down is a no-op: enum-extension migrations cannot be reverted, see the crate doc in lib.rs.

		Ok(())
	}
}
