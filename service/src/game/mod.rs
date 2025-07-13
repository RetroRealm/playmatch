use crate::cache::identify::{
	find_game_and_id_mapping_by_md5_cached, find_game_and_id_mapping_by_sha1_cached,
	find_game_and_id_mapping_by_sha256_cached,
};
use crate::db::game::find_game_and_id_mapping_by_name_and_size;
use crate::manual_match::build_result;
use crate::model::{GameFileMatchSearch, GameMatchResult, GameMatchType};
use sea_orm::DbConn;
use strum::IntoEnumIterator;

pub async fn identify_game(
	search: GameFileMatchSearch,
	conn: &DbConn,
) -> anyhow::Result<GameMatchResult> {
	let mut response_body = None;

	for r#type in GameMatchType::iter() {
		if r#type == GameMatchType::NoMatch {
			continue;
		}

		if let Some((game_release, game_release_id_mappings)) = match r#type {
			GameMatchType::SHA256 => {
				if let Some(sha256) = &search.sha256 {
					find_game_and_id_mapping_by_sha256_cached(sha256, conn).await?
				} else {
					None
				}
			}
			GameMatchType::SHA1 => {
				if let Some(sha1) = &search.sha1 {
					find_game_and_id_mapping_by_sha1_cached(sha1, conn).await?
				} else {
					None
				}
			}
			GameMatchType::MD5 => {
				if let Some(md5) = &search.md5 {
					find_game_and_id_mapping_by_md5_cached(md5, conn).await?
				} else {
					None
				}
			}
			GameMatchType::FileNameAndSize => {
				find_game_and_id_mapping_by_name_and_size(&search.file_name, search.file_size, conn)
					.await?
			}
			GameMatchType::NoMatch => unreachable!(),
		} {
			response_body = Some(build_result(
				r#type,
				game_release,
				game_release_id_mappings,
			)?);
			break;
		}
	}

	Ok(response_body.unwrap_or(GameMatchResult {
		game_match_type: GameMatchType::NoMatch,
		id: None,
		external_metadata: Vec::new(),
	}))
}
