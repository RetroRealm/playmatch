use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let stmt = r#"
            ALTER TABLE signature_metadata_mapping
            ADD CONSTRAINT check_one_id_non_null
            CHECK (num_nonnulls(game_id, company_id, platform_id) = 1);
        "#;
		manager.get_connection().execute_unprepared(stmt).await?;
		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let stmt = r#"
            ALTER TABLE signature_metadata_mapping
            DROP CONSTRAINT IF EXISTS check_one_id_non_null;
        "#;
		manager.get_connection().execute_unprepared(stmt).await?;
		Ok(())
	}
}
