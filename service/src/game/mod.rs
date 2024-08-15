use crate::db::game::{
	find_game_release_and_id_mapping_by_md5, find_game_release_and_id_mapping_by_name_and_size,
	find_game_release_and_id_mapping_by_sha1, find_game_release_and_id_mapping_by_sha256,
};
use crate::model::{GameFileMatchSearch, GameMatchResult, GameMatchResultBuilder, GameMatchType};
use entity::{game_release, game_release_id_mapping};
use sea_orm::DbConn;
use strum::IntoEnumIterator;

pub async fn match_game_if_possible(
	search: GameFileMatchSearch,
	conn: &DbConn,
) -> anyhow::Result<GameMatchResult> {
	let mut response_body = None;

	for r#type in GameMatchType::iter() {
		if r#type == GameMatchType::NoMatch {
			continue;
		}

		if let Some((game_release, game_release_id_mapping)) = match r#type {
			GameMatchType::SHA256 => {
				if let Some(sha256) = &search.sha256 {
					find_game_release_and_id_mapping_by_sha256(sha256, conn).await?
				} else {
					None
				}
			}
			GameMatchType::SHA1 => {
				if let Some(sha1) = &search.sha1 {
					find_game_release_and_id_mapping_by_sha1(sha1, conn).await?
				} else {
					None
				}
			}
			GameMatchType::MD5 => {
				if let Some(md5) = &search.md5 {
					find_game_release_and_id_mapping_by_md5(md5, conn).await?
				} else {
					None
				}
			}
			GameMatchType::FileNameAndSize => {
				find_game_release_and_id_mapping_by_name_and_size(
					&search.file_name,
					search.file_size,
					conn,
				)
				.await?
			}
			GameMatchType::NoMatch => unreachable!(),
		} {
			response_body = Some(build_result(r#type, game_release, game_release_id_mapping)?);
			break;
		}
	}

	Ok(response_body.unwrap_or(GameMatchResult {
		game_match_type: GameMatchType::NoMatch,
		playmatch_id: None,
		igdb_id: None,
		mobygames_id: None,
	}))
}

fn build_result(
	game_match_type: GameMatchType,
	game_release: game_release::Model,
	game_release_id_mapping: game_release_id_mapping::Model,
) -> anyhow::Result<GameMatchResult> {
	Ok(GameMatchResultBuilder::default()
		.game_match_type(game_match_type)
		.igdb_id(Some(game_release_id_mapping.igdb_id))
		.mobygames_id(game_release_id_mapping.moby_games_id)
		.playmatch_id(Some(game_release.id))
		.build()?)
}
