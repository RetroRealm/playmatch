use crate::extension::postgres::Type;
use sea_orm_migration::sea_orm::{EnumIter, Iterable};
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum SignatureGroup {
	Table,
	Id,
	Name,
	WebsiteLink,
	Description,
	CreatedAt,
	UpdatedAt,
}

#[derive(Iden)]
enum DatFile {
	Table,
	Id,
	Name,
	CompanyId,
	PlatformId,
	CurrentVersion,
	SignatureGroupId,
	Tags,
	Subset,
	CreatedAt,
	UpdatedAt,
}

#[derive(Iden)]
enum DatFileImport {
	Table,
	Id,
	DatFileId,
	Name,
	Version,
	Md5Hash,
	ImportedAt,
	CreatedAt,
	UpdatedAt,
}

#[derive(Iden)]
enum Company {
	Table,
	Id,
	Name,
	UpdatedAt,
	CreatedAt,
}

#[derive(Iden)]
enum Platform {
	Table,
	Id,
	Name,
	CompanyId,
	UpdatedAt,
	CreatedAt,
}

#[derive(Iden)]
enum Game {
	Table,
	Id,
	DatFileImportId,
	SignatureGroupInternalId,
	Name,
	Description,
	Categories,
	CloneOf,
	CreatedAt,
	UpdatedAt,
}

#[derive(Iden)]
enum GameFile {
	Table,
	Id,
	GameId,
	FileName,
	FileSizeInBytes,
	Crc,
	Md5,
	Sha1,
	Sha256,
	Status,
	Serial,
	CreatedAt,
	UpdatedAt,
}

#[derive(Iden)]
enum SignatureMetadataMapping {
	Table,
	Id,
	GameId,
	CompanyId,
	PlatformId,
	ProviderName,
	ProviderId,
	MatchType,
	ManualMatchType,
	FailedMatchReason,
	Comment,
	CreatedAt,
	UpdatedAt,
}

#[derive(DeriveIden)]
struct MetadataProviderEnum;

#[derive(DeriveIden, EnumIter)]
pub enum MetadataProvider {
	Igdb,
}

#[derive(DeriveIden)]
struct MatchTypeEnum;

#[derive(DeriveIden, EnumIter)]
pub enum MatchType {
	None,
	Automatic,
	Manual,
	Failed,
}

#[derive(DeriveIden)]
struct ManualMatchModeEnum;

#[derive(DeriveIden, EnumIter)]
pub enum ManualMatchMode {
	Admin,
	Trusted,
	Community,
}

#[derive(DeriveIden)]
struct FailedMatchReasonEnum;

#[derive(DeriveIden, EnumIter)]
pub enum FailedMatchReason {
	NoDirectMatch,
	TooManyMatches,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_type(
				Type::create()
					.as_enum(MetadataProviderEnum)
					.values(MetadataProvider::iter())
					.to_owned(),
			)
			.await?;

		manager
			.create_type(
				Type::create()
					.as_enum(MatchTypeEnum)
					.values(MatchType::iter())
					.to_owned(),
			)
			.await?;

		manager
			.create_type(
				Type::create()
					.as_enum(ManualMatchModeEnum)
					.values(ManualMatchMode::iter())
					.to_owned(),
			)
			.await?;

		manager
			.create_type(
				Type::create()
					.as_enum(FailedMatchReasonEnum)
					.values(FailedMatchReason::iter())
					.to_owned(),
			)
			.await?;

		// Create signature_group table
		manager
			.create_table(
				Table::create()
					.table(SignatureGroup::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(SignatureGroup::Id)
							.uuid()
							.not_null()
							.primary_key()
							.extra("DEFAULT gen_random_uuid()"),
					)
					.col(ColumnDef::new(SignatureGroup::Name).string().not_null())
					.col(ColumnDef::new(SignatureGroup::WebsiteLink).string())
					.col(ColumnDef::new(SignatureGroup::Description).string())
					.col(
						ColumnDef::new(SignatureGroup::CreatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.col(
						ColumnDef::new(SignatureGroup::UpdatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.to_owned(),
			)
			.await?;

		manager
			.create_table(
				Table::create()
					.table(Company::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(Company::Id)
							.uuid()
							.not_null()
							.primary_key()
							.extra("DEFAULT gen_random_uuid()"),
					)
					.col(ColumnDef::new(Company::Name).string().not_null())
					.col(
						ColumnDef::new(Company::UpdatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.col(
						ColumnDef::new(Company::CreatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.to_owned(),
			)
			.await?;

		// Create unique index on company name
		manager
			.create_index(
				Index::create()
					.name("company_name_uindex")
					.table(Company::Table)
					.col(Company::Name)
					.unique()
					.to_owned(),
			)
			.await?;

		// Create platform table
		manager
			.create_table(
				Table::create()
					.table(Platform::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(Platform::Id)
							.uuid()
							.not_null()
							.primary_key()
							.extra("DEFAULT gen_random_uuid()"),
					)
					.col(ColumnDef::new(Platform::Name).string().not_null())
					.col(ColumnDef::new(Platform::CompanyId).uuid().null())
					.col(
						ColumnDef::new(Platform::UpdatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.col(
						ColumnDef::new(Platform::CreatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.foreign_key(
						ForeignKey::create()
							.name("fk-platform-company_id")
							.from(Platform::Table, Platform::CompanyId)
							.to(Company::Table, Company::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.to_owned(),
			)
			.await?;

		// Create unique index on platform name
		manager
			.create_index(
				Index::create()
					.name("platform_name_uindex")
					.table(Platform::Table)
					.col(Platform::Name)
					.unique()
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.name("idx_platform_company_id")
					.table(Platform::Table)
					.col(Platform::CompanyId)
					.to_owned(),
			)
			.await?;

		// Create dat_file table
		manager
			.create_table(
				Table::create()
					.table(DatFile::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(DatFile::Id)
							.uuid()
							.not_null()
							.primary_key()
							.extra("DEFAULT gen_random_uuid()"),
					)
					.col(ColumnDef::new(DatFile::Name).string().not_null())
					.col(ColumnDef::new(DatFile::CompanyId).uuid().null())
					.col(ColumnDef::new(DatFile::PlatformId).uuid().not_null())
					.col(ColumnDef::new(DatFile::CurrentVersion).string().not_null())
					.col(ColumnDef::new(DatFile::SignatureGroupId).uuid().not_null())
					.col(ColumnDef::new(DatFile::Tags).array(ColumnType::Text))
					.col(ColumnDef::new(DatFile::Subset).string().null())
					.col(
						ColumnDef::new(DatFile::CreatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.col(
						ColumnDef::new(DatFile::UpdatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.foreign_key(
						ForeignKey::create()
							.name("fk-dat_file-signature_group_id")
							.from(DatFile::Table, DatFile::SignatureGroupId)
							.to(SignatureGroup::Table, SignatureGroup::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.foreign_key(
						ForeignKey::create()
							.name("fk-dat_file-platform_id")
							.from(DatFile::Table, DatFile::PlatformId)
							.to(Platform::Table, Platform::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.foreign_key(
						ForeignKey::create()
							.name("fk-dat_file-company_id")
							.from(DatFile::Table, DatFile::CompanyId)
							.to(Company::Table, Company::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.to_owned(),
			)
			.await?;

		// Create indexes for dat_file table
		manager
			.create_index(
				Index::create()
					.name("idx_dat_file_name")
					.table(DatFile::Table)
					.col(DatFile::Name)
					.to_owned(),
			)
			.await?;
		manager
			.create_index(
				Index::create()
					.name("idx_dat_file_signature_group_id")
					.table(DatFile::Table)
					.col(DatFile::SignatureGroupId)
					.to_owned(),
			)
			.await?;
		manager
			.create_index(
				Index::create()
					.name("idx_dat_file_company_id")
					.table(DatFile::Table)
					.col(DatFile::CompanyId)
					.to_owned(),
			)
			.await?;
		manager
			.create_index(
				Index::create()
					.name("idx_dat_file_platform_id")
					.table(DatFile::Table)
					.col(DatFile::PlatformId)
					.to_owned(),
			)
			.await?;

		// Create dat_file_import table
		manager
			.create_table(
				Table::create()
					.table(DatFileImport::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(DatFileImport::Id)
							.uuid()
							.not_null()
							.primary_key()
							.extra("DEFAULT gen_random_uuid()"),
					)
					.col(ColumnDef::new(DatFileImport::DatFileId).uuid().not_null())
					.col(ColumnDef::new(DatFileImport::Name).string().not_null())
					.col(ColumnDef::new(DatFileImport::Version).string().not_null())
					.col(ColumnDef::new(DatFileImport::Md5Hash).string().not_null())
					.col(
						ColumnDef::new(DatFileImport::ImportedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.col(
						ColumnDef::new(DatFileImport::CreatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.col(
						ColumnDef::new(DatFileImport::UpdatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.foreign_key(
						ForeignKey::create()
							.name("fk-dat_file_import-dat_file_id")
							.from(DatFileImport::Table, DatFileImport::DatFileId)
							.to(DatFile::Table, DatFile::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.to_owned(),
			)
			.await?;

		// Create indexes for dat_file_import table
		manager
			.create_index(
				Index::create()
					.name("idx_dat_file_import_dat_file_id")
					.table(DatFileImport::Table)
					.col(DatFileImport::DatFileId)
					.to_owned(),
			)
			.await?;
		manager
			.create_index(
				Index::create()
					.name("idx_dat_file_import_version")
					.table(DatFileImport::Table)
					.col(DatFileImport::Version)
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.name("idx_dat_file_import_imported_at")
					.table(DatFileImport::Table)
					.col(DatFileImport::ImportedAt)
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.name("idx_dat_file_import_md5_hash")
					.table(DatFileImport::Table)
					.col(DatFileImport::Md5Hash)
					.to_owned(),
			)
			.await?;

		// Create game table
		manager
			.create_table(
				Table::create()
					.table(Game::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(Game::Id)
							.uuid()
							.not_null()
							.primary_key()
							.extra("DEFAULT gen_random_uuid()"),
					)
					.col(ColumnDef::new(Game::DatFileImportId).uuid().not_null())
					.col(
						ColumnDef::new(Game::SignatureGroupInternalId)
							.string()
							.null(),
					)
					.col(ColumnDef::new(Game::Name).string().not_null())
					.col(ColumnDef::new(Game::Description).string())
					.col(ColumnDef::new(Game::Categories).array(ColumnType::Text))
					.col(ColumnDef::new(Game::CloneOf).uuid().null())
					.col(
						ColumnDef::new(Game::CreatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.col(
						ColumnDef::new(Game::UpdatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.foreign_key(
						ForeignKey::create()
							.name("fk-game-dat_file_import_id")
							.from(Game::Table, Game::DatFileImportId)
							.to(DatFileImport::Table, DatFileImport::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.foreign_key(
						ForeignKey::create()
							.name("fk-game-clone_of")
							.from(Game::Table, Game::CloneOf)
							.to(Game::Table, Game::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.to_owned(),
			)
			.await?;

		// Create indexes for game table
		manager
			.create_index(
				Index::create()
					.name("idx_game_dat_file_import_id")
					.table(Game::Table)
					.col(Game::DatFileImportId)
					.to_owned(),
			)
			.await?;
		manager
			.create_index(
				Index::create()
					.name("idx_game_signature_group_id")
					.table(Game::Table)
					.col(Game::SignatureGroupInternalId)
					.to_owned(),
			)
			.await?;
		manager
			.create_index(
				Index::create()
					.name("idx_game_clone_of")
					.table(Game::Table)
					.col(Game::CloneOf)
					.to_owned(),
			)
			.await?;

		// Create game_file table
		manager
			.create_table(
				Table::create()
					.table(GameFile::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(GameFile::Id)
							.uuid()
							.not_null()
							.primary_key()
							.extra("DEFAULT gen_random_uuid()"),
					)
					.col(ColumnDef::new(GameFile::GameId).uuid().not_null())
					.col(ColumnDef::new(GameFile::FileName).string().not_null())
					.col(ColumnDef::new(GameFile::FileSizeInBytes).big_integer())
					.col(ColumnDef::new(GameFile::Crc).string())
					.col(ColumnDef::new(GameFile::Md5).char_len(32))
					.col(ColumnDef::new(GameFile::Sha1).char_len(40))
					.col(ColumnDef::new(GameFile::Sha256).char_len(64))
					.col(ColumnDef::new(GameFile::Status).string())
					.col(ColumnDef::new(GameFile::Serial).string())
					.col(
						ColumnDef::new(GameFile::CreatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.col(
						ColumnDef::new(GameFile::UpdatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.foreign_key(
						ForeignKey::create()
							.name("fk-game_file-game_id")
							.from(GameFile::Table, GameFile::GameId)
							.to(Game::Table, Game::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.to_owned(),
			)
			.await?;

		// Create indexes for game_file table
		manager
			.create_index(
				Index::create()
					.name("idx_game_file_game_id")
					.table(GameFile::Table)
					.col(GameFile::GameId)
					.to_owned(),
			)
			.await?;
		manager
			.create_index(
				Index::create()
					.name("idx_game_file_sha256")
					.table(GameFile::Table)
					.col(GameFile::Sha256)
					.to_owned(),
			)
			.await?;
		manager
			.create_index(
				Index::create()
					.name("idx_game_file_crc")
					.table(GameFile::Table)
					.col(GameFile::Crc)
					.to_owned(),
			)
			.await?;
		manager
			.create_index(
				Index::create()
					.name("idx_game_file_md5")
					.table(GameFile::Table)
					.col(GameFile::Md5)
					.to_owned(),
			)
			.await?;
		manager
			.create_index(
				Index::create()
					.name("idx_game_file_sha1")
					.table(GameFile::Table)
					.col(GameFile::Sha1)
					.to_owned(),
			)
			.await?;

		// Create signature_metadata_mapping table
		manager
			.create_table(
				Table::create()
					.table(SignatureMetadataMapping::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(SignatureMetadataMapping::Id)
							.uuid()
							.not_null()
							.primary_key()
							.extra("DEFAULT gen_random_uuid()"),
					)
					.col(
						ColumnDef::new(SignatureMetadataMapping::GameId)
							.uuid()
							.null(),
					)
					.col(
						ColumnDef::new(SignatureMetadataMapping::CompanyId)
							.uuid()
							.null(),
					)
					.col(
						ColumnDef::new(SignatureMetadataMapping::PlatformId)
							.uuid()
							.null(),
					)
					.col(enumeration(
						SignatureMetadataMapping::ProviderName,
						MetadataProviderEnum,
						MetadataProvider::iter(),
					))
					.col(
						ColumnDef::new(SignatureMetadataMapping::ProviderId)
							.string()
							.null(),
					)
					.col(enumeration(
						SignatureMetadataMapping::MatchType,
						MatchTypeEnum,
						MatchType::iter(),
					))
					.col(enumeration_null(
						SignatureMetadataMapping::ManualMatchType,
						ManualMatchModeEnum,
						ManualMatchMode::iter(),
					))
					.col(enumeration_null(
						SignatureMetadataMapping::FailedMatchReason,
						FailedMatchReasonEnum,
						FailedMatchReason::iter(),
					))
					.col(ColumnDef::new(SignatureMetadataMapping::Comment).string())
					.col(
						ColumnDef::new(SignatureMetadataMapping::CreatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.col(
						ColumnDef::new(SignatureMetadataMapping::UpdatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::current_timestamp()),
					)
					.foreign_key(
						ForeignKey::create()
							.name("fk-signature_metadata_mapping-game_id")
							.from(
								SignatureMetadataMapping::Table,
								SignatureMetadataMapping::GameId,
							)
							.to(Game::Table, Game::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.foreign_key(
						ForeignKey::create()
							.name("fk-signature_metadata_mapping-company_id")
							.from(
								SignatureMetadataMapping::Table,
								SignatureMetadataMapping::CompanyId,
							)
							.to(Company::Table, Company::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.foreign_key(
						ForeignKey::create()
							.name("fk-signature_metadata_mapping-platform_id")
							.from(
								SignatureMetadataMapping::Table,
								SignatureMetadataMapping::PlatformId,
							)
							.to(Platform::Table, Platform::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.to_owned(),
			)
			.await?;

		// Create indexes for signature_metadata_mapping table
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
			.create_index(
				Index::create()
					.name("idx_signature_metadata_mapping_company_id")
					.table(SignatureMetadataMapping::Table)
					.col(SignatureMetadataMapping::CompanyId)
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
			.await?;

		manager
			.create_index(
				Index::create()
					.name("idx_signature_metadata_mapping_provider")
					.table(SignatureMetadataMapping::Table)
					.col(SignatureMetadataMapping::ProviderName)
					.col(SignatureMetadataMapping::ProviderId)
					.to_owned(),
			)
			.await?;
		manager
			.create_index(
				Index::create()
					.name("idx_signature_metadata_mapping_match_type")
					.table(SignatureMetadataMapping::Table)
					.col(SignatureMetadataMapping::MatchType)
					.to_owned(),
			)
			.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		// Drop tables in reverse order to handle foreign key dependencies correctly
		manager
			.drop_table(
				Table::drop()
					.table(SignatureMetadataMapping::Table)
					.to_owned(),
			)
			.await?;
		manager
			.drop_table(Table::drop().table(GameFile::Table).to_owned())
			.await?;
		manager
			.drop_table(Table::drop().table(Game::Table).to_owned())
			.await?;
		manager
			.drop_table(Table::drop().table(DatFileImport::Table).to_owned())
			.await?;
		manager
			.drop_table(Table::drop().table(DatFile::Table).to_owned())
			.await?;
		manager
			.drop_table(Table::drop().table(Platform::Table).to_owned())
			.await?;
		manager
			.drop_table(Table::drop().table(Company::Table).to_owned())
			.await?;
		manager
			.drop_table(Table::drop().table(SignatureGroup::Table).to_owned())
			.await?;

		manager
			.drop_type(Type::drop().name(MatchTypeEnum).to_owned())
			.await?;

		manager
			.drop_type(Type::drop().name(ManualMatchModeEnum).to_owned())
			.await?;

		manager
			.drop_type(Type::drop().name(FailedMatchReasonEnum).to_owned())
			.await?;

		manager
			.drop_type(Type::drop().name(MetadataProviderEnum).to_owned())
			.await
	}
}
