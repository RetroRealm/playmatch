use crate::db::game::{
	find_game_and_id_mapping_by_md5, find_game_and_id_mapping_by_name_and_size,
	find_game_and_id_mapping_by_sha1, find_game_and_id_mapping_by_sha256,
};
use crate::db::signature_metadata_mapping::{
	create_or_update_signature_metadata_mapping,
	find_signature_metadata_mapping_by_platform_game_company_and_provider,
	SignatureMetadataMappingInputBuilder,
};
use crate::model::{
	GameFileMatchSearch, GameMatchResult, GameMatchResultBuilder, GameMatchType, MatchRequest,
};
use entity::sea_orm_active_enums::MatchTypeEnum;
use entity::{game, signature_metadata_mapping};
use sea_orm::DbConn;
use strum::IntoEnumIterator;

pub async fn apply_manual_game_match(r#match: MatchRequest, conn: &DbConn) -> anyhow::Result<()> {
	let mapping = find_signature_metadata_mapping_by_platform_game_company_and_provider(
		None,
		Some(game_id),
		None,
		r#match.provider.into(),
		conn,
	)
	.await?;

	if let Some(mapping) = mapping {
		if mapping.match_type != MatchTypeEnum::Failed || mapping.match_type != MatchTypeEnum::None
		{
			// TODO: decide how to notify the user that this entry is already matched
		}
	}

	create_or_update_signature_metadata_mapping(
		SignatureMetadataMappingInputBuilder::default()
			.game_id(Some(game_id))
			.provider(r#match.provider.into())
			.provider_id(Some(r#match.provider_id))
			.match_type(MatchTypeEnum::Manual)
			.manual_match_type(Some(r#match.manual_match_type.into()))
			.failed_match_reason(None)
			.automatic_match_reason(None)
			.build()?,
		conn,
	)
	.await?;

	Ok(())
}

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
				.map(Into::into)
				.collect(),
		)
		.build()?;

	Ok(result)
}
