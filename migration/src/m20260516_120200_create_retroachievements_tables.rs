use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum RetroachievementsSystem {
	Table,
	Id,
	SystemId,
	Name,
	CreatedAt,
	UpdatedAt,
}

#[derive(Iden)]
enum RetroachievementsGame {
	Table,
	Id,
	GameId,
	SystemId,
	SystemName,
	Title,
	TitleNormalized,
	ImageIcon,
	NumAchievements,
	NumLeaderboards,
	Points,
	DateModified,
	ForumTopicId,
	CreatedAt,
	UpdatedAt,
}

#[derive(Iden)]
enum RetroachievementsGameHash {
	Table,
	Id,
	GameId,
	Md5,
	CreatedAt,
}

#[derive(Iden)]
enum RetroachievementsImport {
	Table,
	Id,
	ImportedAt,
	SystemCount,
	GameCount,
	HashCount,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_table(
				Table::create()
					.table(RetroachievementsSystem::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(RetroachievementsSystem::Id)
							.uuid()
							.not_null()
							.primary_key()
							.extra("DEFAULT gen_random_uuid()"),
					)
					.col(
						ColumnDef::new(RetroachievementsSystem::SystemId)
							.integer()
							.not_null()
							.unique_key(),
					)
					.col(
						ColumnDef::new(RetroachievementsSystem::Name)
							.text()
							.not_null(),
					)
					.col(
						ColumnDef::new(RetroachievementsSystem::CreatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.col(
						ColumnDef::new(RetroachievementsSystem::UpdatedAt)
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
				"CREATE INDEX IF NOT EXISTS idx_retroachievements_system_name_lower \
				 ON retroachievements_system (lower(name));",
			)
			.await?;

		manager
			.create_table(
				Table::create()
					.table(RetroachievementsGame::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(RetroachievementsGame::Id)
							.uuid()
							.not_null()
							.primary_key()
							.extra("DEFAULT gen_random_uuid()"),
					)
					.col(
						ColumnDef::new(RetroachievementsGame::GameId)
							.big_integer()
							.not_null()
							.unique_key(),
					)
					.col(
						ColumnDef::new(RetroachievementsGame::SystemId)
							.integer()
							.not_null(),
					)
					.col(
						ColumnDef::new(RetroachievementsGame::SystemName)
							.text()
							.not_null(),
					)
					.col(
						ColumnDef::new(RetroachievementsGame::Title)
							.text()
							.not_null(),
					)
					.col(ColumnDef::new(RetroachievementsGame::TitleNormalized).text())
					.col(ColumnDef::new(RetroachievementsGame::ImageIcon).text())
					.col(
						ColumnDef::new(RetroachievementsGame::NumAchievements)
							.integer()
							.not_null()
							.default(0),
					)
					.col(
						ColumnDef::new(RetroachievementsGame::NumLeaderboards)
							.integer()
							.not_null()
							.default(0),
					)
					.col(
						ColumnDef::new(RetroachievementsGame::Points)
							.integer()
							.not_null()
							.default(0),
					)
					.col(ColumnDef::new(RetroachievementsGame::DateModified).text())
					.col(ColumnDef::new(RetroachievementsGame::ForumTopicId).big_integer())
					.col(
						ColumnDef::new(RetroachievementsGame::CreatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.col(
						ColumnDef::new(RetroachievementsGame::UpdatedAt)
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
				"CREATE INDEX IF NOT EXISTS idx_retroachievements_game_title_lower \
				 ON retroachievements_game (lower(title));",
			)
			.await?;
		manager
			.get_connection()
			.execute_unprepared(
				"CREATE INDEX IF NOT EXISTS idx_retroachievements_game_title_normalized_lower \
				 ON retroachievements_game (lower(title_normalized));",
			)
			.await?;
		manager
			.get_connection()
			.execute_unprepared(
				"CREATE INDEX IF NOT EXISTS idx_retroachievements_game_system_title_lower \
				 ON retroachievements_game (lower(system_name), lower(title));",
			)
			.await?;
		manager
			.get_connection()
			.execute_unprepared(
				"CREATE INDEX IF NOT EXISTS idx_retroachievements_game_system_title_normalized_lower \
				 ON retroachievements_game (lower(system_name), lower(title_normalized));",
			)
			.await?;

		manager
			.create_table(
				Table::create()
					.table(RetroachievementsGameHash::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(RetroachievementsGameHash::Id)
							.uuid()
							.not_null()
							.primary_key()
							.extra("DEFAULT gen_random_uuid()"),
					)
					.col(
						ColumnDef::new(RetroachievementsGameHash::GameId)
							.big_integer()
							.not_null(),
					)
					.col(
						ColumnDef::new(RetroachievementsGameHash::Md5)
							.text()
							.not_null(),
					)
					.col(
						ColumnDef::new(RetroachievementsGameHash::CreatedAt)
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
					.name("idx_retroachievements_game_hash_game_id")
					.table(RetroachievementsGameHash::Table)
					.col(RetroachievementsGameHash::GameId)
					.to_owned(),
			)
			.await?;
		manager
			.get_connection()
			.execute_unprepared(
				"CREATE INDEX IF NOT EXISTS idx_retroachievements_game_hash_md5_lower \
				 ON retroachievements_game_hash (lower(md5));",
			)
			.await?;

		manager
			.create_table(
				Table::create()
					.table(RetroachievementsImport::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(RetroachievementsImport::Id)
							.uuid()
							.not_null()
							.primary_key()
							.extra("DEFAULT gen_random_uuid()"),
					)
					.col(
						ColumnDef::new(RetroachievementsImport::ImportedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.col(ColumnDef::new(RetroachievementsImport::SystemCount).integer())
					.col(ColumnDef::new(RetroachievementsImport::GameCount).integer())
					.col(ColumnDef::new(RetroachievementsImport::HashCount).integer())
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.name("idx_retroachievements_import_imported_at")
					.table(RetroachievementsImport::Table)
					.col(RetroachievementsImport::ImportedAt)
					.to_owned(),
			)
			.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.drop_table(
				Table::drop()
					.table(RetroachievementsImport::Table)
					.to_owned(),
			)
			.await?;
		manager
			.drop_table(
				Table::drop()
					.table(RetroachievementsGameHash::Table)
					.to_owned(),
			)
			.await?;
		manager
			.drop_table(Table::drop().table(RetroachievementsGame::Table).to_owned())
			.await?;
		manager
			.drop_table(
				Table::drop()
					.table(RetroachievementsSystem::Table)
					.to_owned(),
			)
			.await?;
		Ok(())
	}
}
