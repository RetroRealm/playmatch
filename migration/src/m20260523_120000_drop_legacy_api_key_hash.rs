use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum User {
	Table,
	ApiKeyHash,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.get_connection()
			.execute_unprepared(r#"DROP INDEX IF EXISTS idx_user_api_key_hash;"#)
			.await?;

		manager
			.alter_table(
				Table::alter()
					.table(User::Table)
					.drop_column(User::ApiKeyHash)
					.to_owned(),
			)
			.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.alter_table(
				Table::alter()
					.table(User::Table)
					.add_column(ColumnDef::new(User::ApiKeyHash).char_len(64).null())
					.to_owned(),
			)
			.await?;

		manager
			.get_connection()
			.execute_unprepared(
				r#"CREATE UNIQUE INDEX idx_user_api_key_hash
                   ON "user" (api_key_hash)
                   WHERE api_key_hash IS NOT NULL;"#,
			)
			.await?;

		Ok(())
	}
}
