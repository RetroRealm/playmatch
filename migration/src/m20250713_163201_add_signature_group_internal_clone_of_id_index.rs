use sea_orm_migration::prelude::*;

#[derive(Iden)]
enum Game {
	Table,
	SignatureGroupInternalCloneOfId,
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_index(
				Index::create()
					.name("game_signature_group_internal_clone_of_id_idx")
					.table(Game::Table)
					.col(Game::SignatureGroupInternalCloneOfId)
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.drop_index(
				Index::drop()
					.name("game_signature_group_internal_clone_of_id_idx")
					.table(Game::Table)
					.to_owned(),
			)
			.await
	}
}
