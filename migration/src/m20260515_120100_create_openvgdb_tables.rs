use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum OpenvgdbRom {
	Table,
	Id,
	RomId,
	SystemId,
	RegionId,
	RomHashCrc,
	RomHashMd5,
	RomHashSha1,
	RomSize,
	RomFileName,
	RomExtensionlessFileName,
	RomSerial,
	CreatedAt,
	UpdatedAt,
}

#[derive(Iden)]
enum OpenvgdbRelease {
	Table,
	Id,
	ReleaseId,
	RomId,
	TitleName,
	RegionName,
	SystemName,
	CoverFront,
	CoverBack,
	Description,
	Developer,
	Publisher,
	Genre,
	ReleaseDate,
	ReleaseYear,
	ReferenceUrl,
	CreatedAt,
}

#[derive(Iden)]
enum OpenvgdbImport {
	Table,
	Id,
	Md5,
	ImportedAt,
	RomCount,
	ReleaseCount,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_table(
				Table::create()
					.table(OpenvgdbRom::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(OpenvgdbRom::Id)
							.uuid()
							.not_null()
							.primary_key()
							.extra("DEFAULT gen_random_uuid()"),
					)
					.col(
						ColumnDef::new(OpenvgdbRom::RomId)
							.big_integer()
							.not_null()
							.unique_key(),
					)
					.col(ColumnDef::new(OpenvgdbRom::SystemId).integer())
					.col(ColumnDef::new(OpenvgdbRom::RegionId).integer())
					.col(ColumnDef::new(OpenvgdbRom::RomHashCrc).text())
					.col(ColumnDef::new(OpenvgdbRom::RomHashMd5).text())
					.col(ColumnDef::new(OpenvgdbRom::RomHashSha1).text())
					.col(ColumnDef::new(OpenvgdbRom::RomSize).big_integer())
					.col(ColumnDef::new(OpenvgdbRom::RomFileName).text())
					.col(ColumnDef::new(OpenvgdbRom::RomExtensionlessFileName).text())
					.col(ColumnDef::new(OpenvgdbRom::RomSerial).text())
					.col(
						ColumnDef::new(OpenvgdbRom::CreatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.col(
						ColumnDef::new(OpenvgdbRom::UpdatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.to_owned(),
			)
			.await?;

		// Hash matching looks up every game file against these three columns,
		// case-insensitively, so a functional index on each keeps it index-only.
		manager
			.get_connection()
			.execute_unprepared(
				"CREATE INDEX IF NOT EXISTS idx_openvgdb_rom_sha1_lower \
				 ON openvgdb_rom (lower(rom_hash_sha1));",
			)
			.await?;
		manager
			.get_connection()
			.execute_unprepared(
				"CREATE INDEX IF NOT EXISTS idx_openvgdb_rom_md5_lower \
				 ON openvgdb_rom (lower(rom_hash_md5));",
			)
			.await?;
		manager
			.get_connection()
			.execute_unprepared(
				"CREATE INDEX IF NOT EXISTS idx_openvgdb_rom_crc_lower \
				 ON openvgdb_rom (lower(rom_hash_crc));",
			)
			.await?;

		manager
			.create_table(
				Table::create()
					.table(OpenvgdbRelease::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(OpenvgdbRelease::Id)
							.uuid()
							.not_null()
							.primary_key()
							.extra("DEFAULT gen_random_uuid()"),
					)
					.col(
						ColumnDef::new(OpenvgdbRelease::ReleaseId)
							.big_integer()
							.not_null()
							.unique_key(),
					)
					.col(
						ColumnDef::new(OpenvgdbRelease::RomId)
							.big_integer()
							.not_null(),
					)
					.col(ColumnDef::new(OpenvgdbRelease::TitleName).text().not_null())
					.col(ColumnDef::new(OpenvgdbRelease::RegionName).text())
					.col(ColumnDef::new(OpenvgdbRelease::SystemName).text())
					.col(ColumnDef::new(OpenvgdbRelease::CoverFront).text())
					.col(ColumnDef::new(OpenvgdbRelease::CoverBack).text())
					.col(ColumnDef::new(OpenvgdbRelease::Description).text())
					.col(ColumnDef::new(OpenvgdbRelease::Developer).text())
					.col(ColumnDef::new(OpenvgdbRelease::Publisher).text())
					.col(ColumnDef::new(OpenvgdbRelease::Genre).text())
					.col(ColumnDef::new(OpenvgdbRelease::ReleaseDate).text())
					.col(ColumnDef::new(OpenvgdbRelease::ReleaseYear).integer())
					.col(ColumnDef::new(OpenvgdbRelease::ReferenceUrl).text())
					.col(
						ColumnDef::new(OpenvgdbRelease::CreatedAt)
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
					.name("idx_openvgdb_release_rom_id")
					.table(OpenvgdbRelease::Table)
					.col(OpenvgdbRelease::RomId)
					.to_owned(),
			)
			.await?;

		manager
			.create_table(
				Table::create()
					.table(OpenvgdbImport::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(OpenvgdbImport::Id)
							.uuid()
							.not_null()
							.primary_key()
							.extra("DEFAULT gen_random_uuid()"),
					)
					.col(
						ColumnDef::new(OpenvgdbImport::Md5)
							.text()
							.not_null()
							.unique_key(),
					)
					.col(
						ColumnDef::new(OpenvgdbImport::ImportedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.col(ColumnDef::new(OpenvgdbImport::RomCount).integer())
					.col(ColumnDef::new(OpenvgdbImport::ReleaseCount).integer())
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.name("idx_openvgdb_import_imported_at")
					.table(OpenvgdbImport::Table)
					.col(OpenvgdbImport::ImportedAt)
					.to_owned(),
			)
			.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.drop_table(Table::drop().table(OpenvgdbImport::Table).to_owned())
			.await?;
		manager
			.drop_table(Table::drop().table(OpenvgdbRelease::Table).to_owned())
			.await?;
		manager
			.drop_table(Table::drop().table(OpenvgdbRom::Table).to_owned())
			.await?;
		Ok(())
	}
}
