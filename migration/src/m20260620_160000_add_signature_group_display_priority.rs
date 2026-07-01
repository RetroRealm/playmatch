use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum SignatureGroup {
	Table,
	DisplayPriority,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.alter_table(
				Table::alter()
					.table(SignatureGroup::Table)
					.add_column_if_not_exists(
						ColumnDef::new(SignatureGroup::DisplayPriority)
							.small_integer()
							.not_null()
							.default(100),
					)
					.to_owned(),
			)
			.await?;

		manager
			.get_connection()
			.execute_unprepared(
				"UPDATE signature_group SET display_priority = CASE name \
				 WHEN 'No-Intro' THEN 10 \
				 WHEN 'Redump' THEN 20 \
				 WHEN 'DatsSite-Legacy' THEN 30 \
				 WHEN 'TOSEC' THEN 50 \
				 WHEN 'MAME' THEN 60 \
				 ELSE display_priority END;",
			)
			.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.alter_table(
				Table::alter()
					.table(SignatureGroup::Table)
					.drop_column(SignatureGroup::DisplayPriority)
					.to_owned(),
			)
			.await?;
		Ok(())
	}
}
