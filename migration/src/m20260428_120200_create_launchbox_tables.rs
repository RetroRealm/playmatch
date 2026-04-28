use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum LaunchboxPlatform {
	Table,
	Id,
	Name,
	Emulated,
	ReleaseDate,
	Developer,
	Manufacturer,
	Cpu,
	Memory,
	Graphics,
	Sound,
	Display,
	Media,
	MaxControllers,
	Notes,
	Category,
	CreatedAt,
	UpdatedAt,
}

#[derive(Iden)]
enum LaunchboxGame {
	Table,
	Id,
	DatabaseId,
	Name,
	PlatformName,
	ReleaseDate,
	ReleaseYear,
	Overview,
	Developer,
	Publisher,
	Genres,
	MaxPlayers,
	Cooperative,
	Esrb,
	ReleaseType,
	Status,
	WikipediaUrl,
	VideoUrl,
	CommunityRating,
	CommunityRatingCount,
	CreatedAt,
	UpdatedAt,
}

#[derive(Iden)]
enum LaunchboxGameAlternateName {
	Table,
	Id,
	LaunchboxGameDatabaseId,
	Name,
	Region,
	CreatedAt,
}

#[derive(Iden)]
enum LaunchboxGameImage {
	Table,
	Id,
	LaunchboxGameDatabaseId,
	FileName,
	ImageType,
	Region,
	CreatedAt,
}

#[derive(Iden)]
enum LaunchboxImport {
	Table,
	Id,
	Md5,
	ImportedAt,
	GameCount,
	PlatformCount,
	AlternateNameCount,
	ImageCount,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_table(
				Table::create()
					.table(LaunchboxPlatform::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(LaunchboxPlatform::Id)
							.uuid()
							.not_null()
							.primary_key()
							.extra("DEFAULT gen_random_uuid()"),
					)
					.col(ColumnDef::new(LaunchboxPlatform::Name).text().not_null())
					.col(ColumnDef::new(LaunchboxPlatform::Emulated).boolean())
					.col(ColumnDef::new(LaunchboxPlatform::ReleaseDate).text())
					.col(ColumnDef::new(LaunchboxPlatform::Developer).text())
					.col(ColumnDef::new(LaunchboxPlatform::Manufacturer).text())
					.col(ColumnDef::new(LaunchboxPlatform::Cpu).text())
					.col(ColumnDef::new(LaunchboxPlatform::Memory).text())
					.col(ColumnDef::new(LaunchboxPlatform::Graphics).text())
					.col(ColumnDef::new(LaunchboxPlatform::Sound).text())
					.col(ColumnDef::new(LaunchboxPlatform::Display).text())
					.col(ColumnDef::new(LaunchboxPlatform::Media).text())
					.col(ColumnDef::new(LaunchboxPlatform::MaxControllers).text())
					.col(ColumnDef::new(LaunchboxPlatform::Notes).text())
					.col(ColumnDef::new(LaunchboxPlatform::Category).text())
					.col(
						ColumnDef::new(LaunchboxPlatform::CreatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.col(
						ColumnDef::new(LaunchboxPlatform::UpdatedAt)
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
					.name("launchbox_platform_name_uindex")
					.table(LaunchboxPlatform::Table)
					.col(LaunchboxPlatform::Name)
					.unique()
					.to_owned(),
			)
			.await?;

		// Case-insensitive lookup is the hot path during platform matching;
		// a functional index on lower(name) keeps it index-only.
		manager
			.get_connection()
			.execute_unprepared(
				"CREATE INDEX IF NOT EXISTS idx_launchbox_platform_name_lower \
				 ON launchbox_platform (lower(name));",
			)
			.await?;

		manager
			.create_table(
				Table::create()
					.table(LaunchboxGame::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(LaunchboxGame::Id)
							.uuid()
							.not_null()
							.primary_key()
							.extra("DEFAULT gen_random_uuid()"),
					)
					.col(
						ColumnDef::new(LaunchboxGame::DatabaseId)
							.big_integer()
							.not_null()
							.unique_key(),
					)
					.col(ColumnDef::new(LaunchboxGame::Name).text().not_null())
					.col(
						ColumnDef::new(LaunchboxGame::PlatformName)
							.text()
							.not_null(),
					)
					.col(ColumnDef::new(LaunchboxGame::ReleaseDate).text())
					.col(ColumnDef::new(LaunchboxGame::ReleaseYear).integer())
					.col(ColumnDef::new(LaunchboxGame::Overview).text())
					.col(ColumnDef::new(LaunchboxGame::Developer).text())
					.col(ColumnDef::new(LaunchboxGame::Publisher).text())
					.col(ColumnDef::new(LaunchboxGame::Genres).text())
					.col(ColumnDef::new(LaunchboxGame::MaxPlayers).integer())
					.col(ColumnDef::new(LaunchboxGame::Cooperative).boolean())
					.col(ColumnDef::new(LaunchboxGame::Esrb).text())
					.col(ColumnDef::new(LaunchboxGame::ReleaseType).text())
					.col(ColumnDef::new(LaunchboxGame::Status).text())
					.col(ColumnDef::new(LaunchboxGame::WikipediaUrl).text())
					.col(ColumnDef::new(LaunchboxGame::VideoUrl).text())
					.col(ColumnDef::new(LaunchboxGame::CommunityRating).float())
					.col(ColumnDef::new(LaunchboxGame::CommunityRatingCount).integer())
					.col(
						ColumnDef::new(LaunchboxGame::CreatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.col(
						ColumnDef::new(LaunchboxGame::UpdatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.to_owned(),
			)
			.await?;

		// Game lookups during matching are platform-scoped + case-insensitive on
		// name, so a single composite functional index covers both the direct
		// and normalised name passes.
		manager
			.get_connection()
			.execute_unprepared(
				"CREATE INDEX IF NOT EXISTS idx_launchbox_game_platform_name_lower \
				 ON launchbox_game (lower(platform_name), lower(name));",
			)
			.await?;

		manager
			.get_connection()
			.execute_unprepared(
				"CREATE INDEX IF NOT EXISTS idx_launchbox_game_name_lower \
				 ON launchbox_game (lower(name));",
			)
			.await?;

		manager
			.create_table(
				Table::create()
					.table(LaunchboxGameAlternateName::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(LaunchboxGameAlternateName::Id)
							.uuid()
							.not_null()
							.primary_key()
							.extra("DEFAULT gen_random_uuid()"),
					)
					.col(
						ColumnDef::new(LaunchboxGameAlternateName::LaunchboxGameDatabaseId)
							.big_integer()
							.not_null(),
					)
					.col(
						ColumnDef::new(LaunchboxGameAlternateName::Name)
							.text()
							.not_null(),
					)
					.col(ColumnDef::new(LaunchboxGameAlternateName::Region).text())
					.col(
						ColumnDef::new(LaunchboxGameAlternateName::CreatedAt)
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
					.name("idx_launchbox_game_alternate_name_database_id")
					.table(LaunchboxGameAlternateName::Table)
					.col(LaunchboxGameAlternateName::LaunchboxGameDatabaseId)
					.to_owned(),
			)
			.await?;

		manager
			.get_connection()
			.execute_unprepared(
				"CREATE INDEX IF NOT EXISTS idx_launchbox_game_alternate_name_name_lower \
				 ON launchbox_game_alternate_name (lower(name));",
			)
			.await?;

		manager
			.create_table(
				Table::create()
					.table(LaunchboxGameImage::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(LaunchboxGameImage::Id)
							.uuid()
							.not_null()
							.primary_key()
							.extra("DEFAULT gen_random_uuid()"),
					)
					.col(
						ColumnDef::new(LaunchboxGameImage::LaunchboxGameDatabaseId)
							.big_integer()
							.not_null(),
					)
					.col(
						ColumnDef::new(LaunchboxGameImage::FileName)
							.text()
							.not_null(),
					)
					.col(
						ColumnDef::new(LaunchboxGameImage::ImageType)
							.text()
							.not_null(),
					)
					.col(ColumnDef::new(LaunchboxGameImage::Region).text())
					.col(
						ColumnDef::new(LaunchboxGameImage::CreatedAt)
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
					.name("idx_launchbox_game_image_database_id")
					.table(LaunchboxGameImage::Table)
					.col(LaunchboxGameImage::LaunchboxGameDatabaseId)
					.to_owned(),
			)
			.await?;

		manager
			.create_table(
				Table::create()
					.table(LaunchboxImport::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(LaunchboxImport::Id)
							.uuid()
							.not_null()
							.primary_key()
							.extra("DEFAULT gen_random_uuid()"),
					)
					.col(
						ColumnDef::new(LaunchboxImport::Md5)
							.text()
							.not_null()
							.unique_key(),
					)
					.col(
						ColumnDef::new(LaunchboxImport::ImportedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.col(ColumnDef::new(LaunchboxImport::GameCount).integer())
					.col(ColumnDef::new(LaunchboxImport::PlatformCount).integer())
					.col(ColumnDef::new(LaunchboxImport::AlternateNameCount).integer())
					.col(ColumnDef::new(LaunchboxImport::ImageCount).integer())
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.name("idx_launchbox_import_imported_at")
					.table(LaunchboxImport::Table)
					.col(LaunchboxImport::ImportedAt)
					.to_owned(),
			)
			.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.drop_table(Table::drop().table(LaunchboxImport::Table).to_owned())
			.await?;
		manager
			.drop_table(Table::drop().table(LaunchboxGameImage::Table).to_owned())
			.await?;
		manager
			.drop_table(
				Table::drop()
					.table(LaunchboxGameAlternateName::Table)
					.to_owned(),
			)
			.await?;
		manager
			.drop_table(Table::drop().table(LaunchboxGame::Table).to_owned())
			.await?;
		manager
			.drop_table(Table::drop().table(LaunchboxPlatform::Table).to_owned())
			.await?;
		Ok(())
	}
}
