use crate::sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::prelude::Uuid;
use sha2::{Digest, Sha256};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum User {
	Table,
	ApiKey,
	ApiKeyHash,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let conn = manager.get_connection();

		manager
			.alter_table(
				Table::alter()
					.table(User::Table)
					.add_column(ColumnDef::new(User::ApiKeyHash).char_len(64).null())
					.to_owned(),
			)
			.await?;

		// Hashing runs in Rust so the migration does not depend on pgcrypto.
		let rows = conn
			.query_all(Statement::from_string(
				DbBackend::Postgres,
				r#"SELECT id, api_key FROM "user" WHERE api_key IS NOT NULL"#,
			))
			.await?;

		for row in rows {
			let id: Uuid = row.try_get("", "id")?;
			let api_key: String = row.try_get("", "api_key")?;
			let hash_hex = hex::encode(Sha256::digest(api_key.as_bytes()));

			conn.execute(Statement::from_sql_and_values(
				DbBackend::Postgres,
				r#"UPDATE "user" SET api_key_hash = $1 WHERE id = $2"#,
				[hash_hex.into(), id.into()],
			))
			.await?;
		}

		conn.execute_unprepared(
			r#"CREATE UNIQUE INDEX idx_user_api_key_hash ON "user" (api_key_hash) WHERE api_key_hash IS NOT NULL;"#,
		)
		.await?;

		manager
			.alter_table(
				Table::alter()
					.table(User::Table)
					.drop_column(User::ApiKey)
					.to_owned(),
			)
			.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let conn = manager.get_connection();

		manager
			.alter_table(
				Table::alter()
					.table(User::Table)
					.add_column(ColumnDef::new(User::ApiKey).string().null())
					.to_owned(),
			)
			.await?;

		conn.execute_unprepared(r#"DROP INDEX IF EXISTS idx_user_api_key_hash;"#)
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
}
