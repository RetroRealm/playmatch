use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum TgdbGame {
	Table,
	Id,
	Title,
	TitleNormalized,
	CreatedAt,
	UpdatedAt,
}

#[derive(Iden)]
enum TgdbGameAlias {
	Table,
	Id,
	GameId,
	AltName,
	AltNameNormalized,
	CreatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_table(
				Table::create()
					.table(TgdbGame::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(TgdbGame::Id)
							.big_integer()
							.not_null()
							.primary_key(),
					)
					.col(ColumnDef::new(TgdbGame::Title).text().not_null())
					.col(ColumnDef::new(TgdbGame::TitleNormalized).text())
					.col(
						ColumnDef::new(TgdbGame::CreatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.col(
						ColumnDef::new(TgdbGame::UpdatedAt)
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
				"CREATE INDEX IF NOT EXISTS idx_tgdb_game_title_lower \
				 ON tgdb_game (lower(title));",
			)
			.await?;
		manager
			.get_connection()
			.execute_unprepared(
				"CREATE INDEX IF NOT EXISTS idx_tgdb_game_title_normalized_lower \
				 ON tgdb_game (lower(title_normalized));",
			)
			.await?;

		manager
			.create_table(
				Table::create()
					.table(TgdbGameAlias::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(TgdbGameAlias::Id)
							.uuid()
							.not_null()
							.primary_key()
							.extra("DEFAULT gen_random_uuid()"),
					)
					.col(
						ColumnDef::new(TgdbGameAlias::GameId)
							.big_integer()
							.not_null(),
					)
					.col(ColumnDef::new(TgdbGameAlias::AltName).text().not_null())
					.col(ColumnDef::new(TgdbGameAlias::AltNameNormalized).text())
					.col(
						ColumnDef::new(TgdbGameAlias::CreatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.name("idx_tgdb_game_alias_game_id")
					.table(TgdbGameAlias::Table)
					.col(TgdbGameAlias::GameId)
					.to_owned(),
			)
			.await?;
		manager
			.get_connection()
			.execute_unprepared(
				"CREATE INDEX IF NOT EXISTS idx_tgdb_game_alias_alt_name_lower \
				 ON tgdb_game_alias (lower(alt_name));",
			)
			.await?;
		manager
			.get_connection()
			.execute_unprepared(
				"CREATE INDEX IF NOT EXISTS idx_tgdb_game_alias_alt_name_normalized_lower \
				 ON tgdb_game_alias (lower(alt_name_normalized));",
			)
			.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.drop_table(Table::drop().table(TgdbGameAlias::Table).to_owned())
			.await?;
		manager
			.drop_table(Table::drop().table(TgdbGame::Table).to_owned())
			.await?;
		Ok(())
	}
}
