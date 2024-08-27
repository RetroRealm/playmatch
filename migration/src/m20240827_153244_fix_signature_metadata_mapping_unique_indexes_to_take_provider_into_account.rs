use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum SignatureMetadataMapping {
	Table,
	GameId,
	CompanyId,
	PlatformId,
	Provider,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.drop_index(
				Index::drop()
					.name("idx_signature_metadata_mapping_game_id_unique")
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.name("idx_signature_metadata_mapping_game_id")
					.table(SignatureMetadataMapping::Table)
					.col(SignatureMetadataMapping::GameId)
					.to_owned(),
			)
			.await?;

		manager
			.drop_index(
				Index::drop()
					.name("idx_signature_metadata_mapping_company_id_unique")
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.name("idx_signature_metadata_mapping_company_id")
					.table(SignatureMetadataMapping::Table)
					.col(SignatureMetadataMapping::CompanyId)
					.to_owned(),
			)
			.await?;

		manager
			.drop_index(
				Index::drop()
					.name("idx_signature_metadata_mapping_platform_id_unique")
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.name("idx_signature_metadata_mapping_platform_id")
					.table(SignatureMetadataMapping::Table)
					.col(SignatureMetadataMapping::PlatformId)
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.name("signature_metadata_mapping_company_id_provider_uindex")
					.table(SignatureMetadataMapping::Table)
					.col(SignatureMetadataMapping::CompanyId)
					.col(SignatureMetadataMapping::Provider)
					.unique()
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.name("signature_metadata_mapping_game_id_provider_uindex")
					.table(SignatureMetadataMapping::Table)
					.col(SignatureMetadataMapping::GameId)
					.col(SignatureMetadataMapping::Provider)
					.unique()
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.name("signature_metadata_mapping_provider_platform_id_uindex")
					.table(SignatureMetadataMapping::Table)
					.col(SignatureMetadataMapping::Provider)
					.col(SignatureMetadataMapping::PlatformId)
					.unique()
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.drop_index(
				Index::drop()
					.name("signature_metadata_mapping_company_id_provider_uindex")
					.to_owned(),
			)
			.await?;

		manager
			.drop_index(
				Index::drop()
					.name("signature_metadata_mapping_game_id_provider_uindex")
					.to_owned(),
			)
			.await?;

		manager
			.drop_index(
				Index::drop()
					.name("signature_metadata_mapping_provider_platform_id_uindex")
					.to_owned(),
			)
			.await?;

		manager
			.drop_index(
				Index::drop()
					.name("idx_signature_metadata_mapping_game_id")
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.name("idx_signature_metadata_mapping_game_id_unique")
					.table(SignatureMetadataMapping::Table)
					.col(SignatureMetadataMapping::GameId)
					.unique()
					.to_owned(),
			)
			.await?;

		manager
			.drop_index(
				Index::drop()
					.name("idx_signature_metadata_mapping_company_id")
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.name("idx_signature_metadata_mapping_company_id_unique")
					.table(SignatureMetadataMapping::Table)
					.col(SignatureMetadataMapping::CompanyId)
					.unique()
					.to_owned(),
			)
			.await?;

		manager
			.drop_index(
				Index::drop()
					.name("idx_signature_metadata_mapping_platform_id")
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.name("idx_signature_metadata_mapping_platform_id_unique")
					.table(SignatureMetadataMapping::Table)
					.col(SignatureMetadataMapping::PlatformId)
					.unique()
					.to_owned(),
			)
			.await
	}
}
