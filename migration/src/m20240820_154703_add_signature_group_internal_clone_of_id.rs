use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum Game {
	Table,
	SignatureGroupInternalCloneOfId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.alter_table(
				TableAlterStatement::new()
					.table(Game::Table)
					.add_column(
						ColumnDef::new(Game::SignatureGroupInternalCloneOfId)
							.text()
							.null(),
					)
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.alter_table(
				TableAlterStatement::new()
					.table(Game::Table)
					.drop_column(Game::SignatureGroupInternalCloneOfId)
					.to_owned(),
			)
			.await
	}
}
