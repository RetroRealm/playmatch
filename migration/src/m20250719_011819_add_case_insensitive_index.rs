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
				r#"
                CREATE INDEX IF NOT EXISTS idx_company_lower_name
                ON company ((LOWER("name")));
                "#,
			)
			.await?;

		manager
			.get_connection()
			.execute_unprepared(
				r#"
                CREATE INDEX IF NOT EXISTS idx_platform_lower_name
                ON platform ((LOWER("name")));
                "#,
			)
			.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.get_connection()
			.execute_unprepared(r#"DROP INDEX IF EXISTS idx_company_lower_name;"#)
			.await?;

		manager
			.get_connection()
			.execute_unprepared(r#"DROP INDEX IF EXISTS idx_platform_lower_name;"#)
			.await?;

		Ok(())
	}
}
