use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum User {
	Table,
	ApiKeyHashHmac,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		// No backfill: SHA-256 is not reversible, so the auth path promotes rows
		// from the legacy column to this one on their next successful login.
		manager
			.alter_table(
				Table::alter()
					.table(User::Table)
					.add_column(ColumnDef::new(User::ApiKeyHashHmac).char_len(64).null())
					.to_owned(),
			)
			.await?;

		manager
			.get_connection()
			.execute_unprepared(
				r#"CREATE UNIQUE INDEX idx_user_api_key_hash_hmac
                   ON "user" (api_key_hash_hmac)
                   WHERE api_key_hash_hmac IS NOT NULL;"#,
			)
			.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.get_connection()
			.execute_unprepared(r#"DROP INDEX IF EXISTS idx_user_api_key_hash_hmac;"#)
			.await?;

		manager
			.alter_table(
				Table::alter()
					.table(User::Table)
					.drop_column(User::ApiKeyHashHmac)
					.to_owned(),
			)
			.await?;

		Ok(())
	}
}
