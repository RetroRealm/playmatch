use sea_orm::prelude::Uuid;
use sea_orm::{
	sea_query::SimpleExpr, ActiveModelTrait, ActiveValue::Set, ColumnTrait, DbConn, DbErr,
	EntityTrait, QueryFilter,
};

use crate::dat::shared::model::{Game, RomElement};
use entity::sea_orm_active_enums::GameReleaseProviderEnum;
use ::entity::{
	game_file, game_file::Entity as GameFile, game_release, game_release::Entity as GameRelease,
	game_release_id_mapping, game_release_id_mapping::Entity as GameReleaseIdMapping,
};

pub async fn insert_game_file(
    game_file: RomElement,
    game_release_id: Uuid,
    conn: &DbConn,
) -> anyhow::Result<game_file::ActiveModel> {
    let game_file = game_file::ActiveModel {
        name: Set(game_file.name),
        size: Set(game_file.size.parse()?),
        crc: Set(Some(game_file.crc)),
        md5: Set(game_file.md5),
        sha1: Set(game_file.sha1),
        sha256: Set(game_file.sha256),
        status: Set(game_file.status.map(|s| s.to_string())),
        serial: Set(game_file.serial),
        game_release_id: Set(game_release_id),
        ..Default::default()
    };

    game_file.save(conn).await.map_err(|e| e.into())
}

pub async fn insert_game_release(
    release_provider: GameReleaseProviderEnum,
    company: String,
    platform: String,
    game: Game,
    conn: &DbConn,
) -> Result<game_release::ActiveModel, DbErr> {
    let game_release = game_release::ActiveModel {
        game_release_provider: Set(release_provider),
        platform_company: Set(Some(company)),
        platform: Set(Some(platform)),
        game_id: Set(game.id),
        name: Set(game.name),
        description: Set(game.description),
        categories: Set(game.category),
        ..Default::default()
    };

    game_release.save(conn).await
}

pub async fn find_game_release_by_name_and_platform_and_platform_company(
    name: &str,
    platform: &str,
    platform_company: &str,
    conn: &DbConn,
) -> Result<Option<game_release::Model>, DbErr> {
    GameRelease::find()
        .filter(
            game_release::Column::Name
                .eq(name)
                .and(game_release::Column::Platform.eq(platform))
                .and(game_release::Column::PlatformCompany.eq(platform_company)),
        )
        .one(conn)
        .await
}

pub async fn find_game_release_and_id_mapping_by_md5(
    md5: &str,
    conn: &DbConn,
) -> Result<Option<(game_release::Model, game_release_id_mapping::Model)>, DbErr> {
    find_game_release_id_mapping_if_exists_by_filter(game_file::Column::Md5.eq(md5), conn).await
}

pub async fn find_game_release_and_id_mapping_by_sha1(
    sha1: &str,
    conn: &DbConn,
) -> Result<Option<(game_release::Model, game_release_id_mapping::Model)>, DbErr> {
    find_game_release_id_mapping_if_exists_by_filter(game_file::Column::Sha1.eq(sha1), conn).await
}

pub async fn find_game_release_and_id_mapping_by_sha256(
    sha256: &str,
    conn: &DbConn,
) -> Result<Option<(game_release::Model, game_release_id_mapping::Model)>, DbErr> {
    find_game_release_id_mapping_if_exists_by_filter(game_file::Column::Sha256.eq(sha256), conn)
        .await
}

pub async fn find_game_release_and_id_mapping_by_name_and_size(
    name: &str,
    size: i64,
    conn: &DbConn,
) -> Result<Option<(game_release::Model, game_release_id_mapping::Model)>, DbErr> {
    find_game_release_id_mapping_if_exists_by_filter(
        game_file::Column::Name
            .eq(name)
            .and(game_file::Column::Size.eq(size)),
        conn,
    )
    .await
}

async fn find_game_release_id_mapping_if_exists_by_filter(
    input: SimpleExpr,
    conn: &DbConn,
) -> Result<Option<(game_release::Model, game_release_id_mapping::Model)>, DbErr> {
    let game_file = GameFile::find()
        .filter(input)
        .find_also_related(GameRelease)
        .one(conn)
        .await?;

    match game_file {
        Some((_, Some(game_release))) => {
            let game_release_id_mapping = GameReleaseIdMapping::find()
                .filter(game_release_id_mapping::Column::GameReleaseId.eq(game_release.id))
                .one(conn)
                .await?;

            if let Some(game_release_id_mapping) = game_release_id_mapping {
                Ok(Some((game_release, game_release_id_mapping)))
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}
