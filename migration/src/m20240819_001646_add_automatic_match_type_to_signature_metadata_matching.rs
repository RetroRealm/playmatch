use crate::extension::postgres::Type;
use crate::m20240816_000001_initial_migration::FailedMatchReason;
use crate::sea_orm::{EnumIter, Iterable};
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
struct AutomaticMatchReasonEnum;

#[derive(DeriveIden, EnumIter)]
pub enum AutomaticMatchReason {
	DirectName,
	AlternativeName,
}

#[derive(Iden)]
enum SignatureMetadataMapping {
	Table,
	AutomaticMatchReason,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_type(
				Type::create()
					.as_enum(AutomaticMatchReasonEnum)
					.values(AutomaticMatchReason::iter())
					.to_owned(),
			)
			.await?;

		manager
			.alter_table(
				Table::alter()
					.table(SignatureMetadataMapping::Table)
					.add_column(
						ColumnDef::new(SignatureMetadataMapping::AutomaticMatchReason)
							.enumeration(AutomaticMatchReasonEnum, AutomaticMatchReason::iter())
							.null()
							.to_owned(),
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
					.drop_column(SignatureMetadataMapping::AutomaticMatchReason)
					.to_owned(),
			)
			.await?;

		manager
			.drop_type(Type::drop().name(AutomaticMatchReasonEnum).to_owned())
			.await
	}
}

#[derive(DeriveIden)]
enum Post {
	Table,
	Id,
	Title,
	Text,
}
