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
		// This migration does not support down migration as it adds a new enum value and postgresql doesn't allow to delete enum values.

		Ok(())
	}
}
