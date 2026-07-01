use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum ContentAnchor {
	Table,
	Id,
	AnchorHash,
	AnchorFileCount,
	CreatedAt,
	UpdatedAt,
}

#[derive(Iden)]
enum Game {
	Table,
	ContentAnchorId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_table(
				Table::create()
					.table(ContentAnchor::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(ContentAnchor::Id)
							.uuid()
							.not_null()
							.primary_key()
							.extra("DEFAULT gen_random_uuid()"),
					)
					.col(ColumnDef::new(ContentAnchor::AnchorHash).text().not_null())
					.col(
						ColumnDef::new(ContentAnchor::AnchorFileCount)
							.small_integer()
							.not_null(),
					)
					.col(
						ColumnDef::new(ContentAnchor::CreatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.col(
						ColumnDef::new(ContentAnchor::UpdatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.to_owned(),
			)
			.await?;

		manager
			.get_connection()
			.execute_unprepared(
				"CREATE UNIQUE INDEX IF NOT EXISTS idx_content_anchor_lower_anchor_hash \
				 ON content_anchor (LOWER(anchor_hash));",
			)
			.await?;

		manager
			.alter_table(
				Table::alter()
					.table(Game::Table)
					.add_column_if_not_exists(ColumnDef::new(Game::ContentAnchorId).uuid().null())
					.add_foreign_key(
						&TableForeignKey::new()
							.name("fk_game_content_anchor_id")
							.from_tbl(Game::Table)
							.from_col(Game::ContentAnchorId)
							.to_tbl(ContentAnchor::Table)
							.to_col(ContentAnchor::Id)
							.on_delete(ForeignKeyAction::SetNull)
							.to_owned(),
					)
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.if_not_exists()
					.name("idx_game_content_anchor_id")
					.table(Game::Table)
					.col(Game::ContentAnchorId)
					.to_owned(),
			)
			.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.drop_index(
				Index::drop()
					.if_exists()
					.name("idx_game_content_anchor_id")
					.table(Game::Table)
					.to_owned(),
			)
			.await?;

		manager
			.alter_table(
				Table::alter()
					.table(Game::Table)
					.drop_foreign_key("fk_game_content_anchor_id")
					.drop_column(Game::ContentAnchorId)
					.to_owned(),
			)
			.await?;

		manager
			.drop_table(
				Table::drop()
					.if_exists()
					.table(ContentAnchor::Table)
					.to_owned(),
			)
			.await?;

		Ok(())
	}
}
