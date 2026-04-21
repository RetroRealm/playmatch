use sea_orm_migration::prelude::*;

#[derive(Iden)]
enum SignatureMetadataMappingSuggestions {
	Table,
	Source,
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.alter_table(
				Table::alter()
					.table(SignatureMetadataMappingSuggestions::Table)
					.add_column(
						ColumnDef::new(SignatureMetadataMappingSuggestions::Source)
							.string_len(255)
							.null(),
					)
					.to_owned(),
			)
			.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.alter_table(
				Table::alter()
					.table(SignatureMetadataMappingSuggestions::Table)
					.drop_column(SignatureMetadataMappingSuggestions::Source)
					.to_owned(),
			)
			.await?;

		Ok(())
	}
}
