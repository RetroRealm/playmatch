use sea_orm::*;
use sea_orm::sea_query::SimpleExpr;

use ::entity::{
	game_file, game_file::Entity as GameFile, game_release, game_release::Entity as GameRelease,
	game_release_id_mapping, game_release_id_mapping::Entity as GameReleaseIdMapping,
};

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
        None => Ok(None),
        Some((_, game_release)) => match game_release {
            None => Ok(None),
            Some(game_release) => {
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
        },
    }
}
