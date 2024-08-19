use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum SignatureMetadataMapping {
	Table,
	ProviderName,
	Provider,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.alter_table(
				Table::alter()
					.table(SignatureMetadataMapping::Table)
					.rename_column(
						SignatureMetadataMapping::ProviderName,
						SignatureMetadataMapping::Provider,
					)
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.alter_table(
				Table::alter()
					.table(SignatureMetadataMapping::Table)
					.rename_column(
						SignatureMetadataMapping::Provider,
						SignatureMetadataMapping::ProviderName,
					)
					.to_owned(),
			)
			.await
	}
}
