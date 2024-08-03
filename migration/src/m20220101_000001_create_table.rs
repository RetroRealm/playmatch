use sea_orm_migration::prelude::*;

use crate::extension::postgres::Type;
use crate::sea_orm::{EnumIter, Iterable};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
struct GameReleaseProviderEnum;

#[derive(DeriveIden, EnumIter)]
enum GameReleaseProviderVariants {
    NoIntro,
    Redump,
}

#[derive(DeriveIden)]
enum GameRelease {
    Table,
    Id,
    GameReleaseProvider,
    PlatformCompany,
    Platform,
    GameID,
    Name,
    Description,
    Categories,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum GameFile {
    Table,
    Id,
    Name,
    Size,
    CRC,
    MD5,
    SHA1,
    SHA256,
    Status,
    Serial,
    GameReleaseId,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum GameReleaseIdMapping {
    Table,
    GameReleaseId,
    IgdbId,
    MobyGamesId,
    Comment,
    CreatedAt,
    UpdatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create the enum type for GameReleaseProvider
        manager
            .create_type(
                Type::create()
                    .as_enum(GameReleaseProviderEnum)
                    .values(GameReleaseProviderVariants::iter())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(GameRelease::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GameRelease::Id)
                            .uuid()
                            .not_null()
                            .extra("DEFAULT gen_random_uuid()")
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(GameRelease::GameReleaseProvider)
                            .enumeration(
                                GameReleaseProviderEnum,
                                GameReleaseProviderVariants::iter(),
                            )
                            .not_null(),
                    )
                    .col(ColumnDef::new(GameRelease::PlatformCompany).text())
                    .col(ColumnDef::new(GameRelease::Platform).text())
                    .col(ColumnDef::new(GameRelease::GameID).text().not_null())
                    .col(ColumnDef::new(GameRelease::Name).text().not_null())
                    .col(ColumnDef::new(GameRelease::Description).text())
                    .col(ColumnDef::new(GameRelease::Categories).array(ColumnType::Text))
                    .col(
                        ColumnDef::new(GameRelease::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default("now()"),
                    )
                    .col(
                        ColumnDef::new(GameRelease::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default("now()"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(GameRelease::Table)
                    .col(GameRelease::GameID)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(GameRelease::Table)
                    .col(GameRelease::Name)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(GameFile::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GameFile::Id)
                            .uuid()
                            .not_null()
                            .extra("DEFAULT gen_random_uuid()")
                            .primary_key(),
                    )
                    .col(ColumnDef::new(GameFile::Name).text().not_null())
                    .col(ColumnDef::new(GameFile::Size).big_integer())
                    .col(ColumnDef::new(GameFile::CRC).text())
                    .col(ColumnDef::new(GameFile::MD5).char_len(32))
                    .col(ColumnDef::new(GameFile::SHA1).char_len(40))
                    .col(ColumnDef::new(GameFile::SHA256).char_len(64))
                    .col(ColumnDef::new(GameFile::Status).text())
                    .col(ColumnDef::new(GameFile::Serial).text())
                    .col(ColumnDef::new(GameFile::GameReleaseId).uuid().not_null())
                    .col(
                        ColumnDef::new(GameFile::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default("now()"),
                    )
                    .col(
                        ColumnDef::new(GameFile::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default("now()"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk-game_file-game_release_id")
                    .from(GameFile::Table, GameFile::GameReleaseId)
                    .to(GameRelease::Table, GameRelease::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(GameFile::Table)
                    .col(GameFile::Name)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(GameFile::Table)
                    .col(GameFile::Size)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(GameFile::Table)
                    .col(GameFile::CRC)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(GameFile::Table)
                    .col(GameFile::MD5)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(GameFile::Table)
                    .col(GameFile::SHA1)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(GameFile::Table)
                    .col(GameFile::SHA256)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(GameReleaseIdMapping::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GameReleaseIdMapping::GameReleaseId)
                            .uuid()
                            .primary_key()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GameReleaseIdMapping::IgdbId)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(GameReleaseIdMapping::MobyGamesId).integer())
                    .col(ColumnDef::new(GameReleaseIdMapping::Comment).text())
                    .col(
                        ColumnDef::new(GameReleaseIdMapping::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default("now()"),
                    )
                    .col(
                        ColumnDef::new(GameReleaseIdMapping::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default("now()"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk-game_release_id_mapping-game_release_id")
                    .from(
                        GameReleaseIdMapping::Table,
                        GameReleaseIdMapping::GameReleaseId,
                    )
                    .to(GameRelease::Table, GameRelease::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(GameReleaseIdMapping::Table)
                    .col(GameReleaseIdMapping::GameReleaseId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(GameReleaseIdMapping::Table)
                    .col(GameReleaseIdMapping::IgdbId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(GameReleaseIdMapping::Table)
                    .col(GameReleaseIdMapping::MobyGamesId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(GameReleaseIdMapping::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(GameFile::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(GameRelease::Table).to_owned())
            .await?;

        manager
            .drop_type(Type::drop().name(GameReleaseProviderEnum).to_owned())
            .await
    }
}
