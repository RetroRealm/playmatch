use crate::sea_orm::{ColumnTrait, DbBackend, EntityTrait, QueryFilter, Statement};
use entity::signature_metadata_mapping;
use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::prelude::Uuid;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum SignatureMetadataMapping {
	Table,
	GameId,
	PlatformId,
	CompanyId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let conn = manager.get_connection();

		let ids_to_delete = signature_metadata_mapping::Entity::find()
			.from_raw_sql(Statement::from_sql_and_values(
				DbBackend::Postgres, // Use the appropriate DbBackend (e.g., Postgres, MySql, etc.)
				r#"
                SELECT a.id FROM signature_metadata_mapping a
                INNER JOIN signature_metadata_mapping b
                ON a.game_id = b.game_id AND a.id < b.id
            "#,
				vec![],
			))
			.all(conn)
			.await?
			.into_iter()
			.map(|result| result.id)
			.collect::<Vec<Uuid>>();

		signature_metadata_mapping::Entity::delete_many()
			.filter(signature_metadata_mapping::Column::Id.is_in(ids_to_delete))
			.exec(conn)
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

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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
			.await
	}
}
