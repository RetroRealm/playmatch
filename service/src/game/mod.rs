use crate::db::game::{
	find_game_and_id_mapping_by_md5, find_game_and_id_mapping_by_name_and_size,
	find_game_and_id_mapping_by_sha1, find_game_and_id_mapping_by_sha256,
};
use crate::model::{
	ExternalMetadata, GameFileMatchSearch, GameMatchResult, GameMatchResultBuilder, GameMatchType,
};
use entity::{game, signature_metadata_mapping};
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

		if let Some((game_release, game_release_id_mappings)) = match r#type {
			GameMatchType::SHA256 => {
				if let Some(sha256) = &search.sha256 {
					find_game_and_id_mapping_by_sha256(sha256, conn).await?
				} else {
					None
				}
			}
			GameMatchType::SHA1 => {
				if let Some(sha1) = &search.sha1 {
					find_game_and_id_mapping_by_sha1(sha1, conn).await?
				} else {
					None
				}
			}
			GameMatchType::MD5 => {
				if let Some(md5) = &search.md5 {
					find_game_and_id_mapping_by_md5(md5, conn).await?
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

fn build_result(
	game_match_type: GameMatchType,
	game: game::Model,
	signature_metadata_mappings: Vec<signature_metadata_mapping::Model>,
) -> anyhow::Result<GameMatchResult> {
	let result = GameMatchResultBuilder::default()
		.game_match_type(game_match_type)
		.id(Some(game.id))
		.external_metadata(
			signature_metadata_mappings
				.into_iter()
				.map(|mapping| ExternalMetadata {
					provider_name: mapping.provider.into(),
					provider_id: mapping.provider_id,
					match_type: mapping.match_type.into(),
					comment: mapping.comment,
					manual_match_type: mapping.manual_match_type.map(Into::into),
					failed_match_reason: mapping.failed_match_reason.map(Into::into),
					automatic_match_reason: mapping.automatic_match_reason.map(Into::into),
				})
				.collect(),
		)
		.build()?;

	Ok(result)
}
