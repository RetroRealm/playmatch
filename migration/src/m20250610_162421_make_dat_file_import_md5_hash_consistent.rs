use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum DatFileImport {
	Table,
	MD5Hash,
	MD5,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.alter_table(
				Table::alter()
					.table(DatFileImport::Table)
					.rename_column(DatFileImport::MD5Hash, DatFileImport::MD5)
					.to_owned(),
			)
			.await?;

		manager
			.alter_table(
				Table::alter()
					.table(DatFileImport::Table)
					.modify_column(ColumnDef::new(DatFileImport::MD5).char_len(32).not_null())
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.alter_table(
				Table::alter()
					.table(DatFileImport::Table)
					.rename_column(DatFileImport::MD5, DatFileImport::MD5Hash)
					.to_owned(),
			)
			.await?;

		manager
			.alter_table(
				Table::alter()
					.table(DatFileImport::Table)
					.modify_column(ColumnDef::new(DatFileImport::MD5Hash).text().not_null())
					.to_owned(),
			)
			.await
	}
}
