use crate::db::game::{
	find_game_and_id_mapping_by_md5, find_game_and_id_mapping_by_name_and_size,
	find_game_and_id_mapping_by_sha1, find_game_and_id_mapping_by_sha256,
	find_game_by_name_or_game_file_name, find_game_signature_metadata_mapping,
	find_games_by_name_and_platform_id,
};
use crate::db::platform::find_platform_of_game;
use crate::db::signature_metadata_mapping::{
	create_or_update_signature_metadata_mapping, SignatureMetadataMappingInputBuilder,
};
use crate::error::{ServiceError, ServiceResult};
use crate::model::{
	GameFileMatchSearch, GameMatchResult, GameMatchResultBuilder, GameMatchType, MatchRequest,
	UpdatedMatchResult, UpdatedMatchResultBuilder,
};
use entity::sea_orm_active_enums::MatchTypeEnum;
use entity::{game, signature_metadata_mapping};
use log::debug;
use sea_orm::DbConn;
use strum::IntoEnumIterator;

pub async fn apply_manual_game_match(
	r#match: MatchRequest,
	conn: &DbConn,
) -> ServiceResult<Vec<UpdatedMatchResult>> {
	let found_game = if let Some(sha256) = &r#match.sha256 {
		find_game_and_id_mapping_by_sha256(sha256, conn)
			.await?
			.and_then(|(game, _)| Some(game))
	} else if let Some(sha1) = &r#match.sha1 {
		find_game_and_id_mapping_by_sha1(sha1, conn)
			.await?
			.and_then(|(game, _)| Some(game))
	} else if let Some(md5) = &r#match.md5 {
		find_game_and_id_mapping_by_md5(md5, conn)
			.await?
			.and_then(|(game, _)| Some(game))
	} else if let Some(file_name) = &r#match.name {
		find_game_by_name_or_game_file_name(file_name, conn).await?
	} else {
		None
	};

	let game = found_game.ok_or(ServiceError::GameNotFound)?;
	let platform = find_platform_of_game(game.id, conn).await?;

	let games = if let Some(platform) = platform {
		find_games_by_name_and_platform_id(&game.name, platform.id, conn).await?
	} else {
		vec![game]
	};

	let mut results = vec![];

	for game in games {
		let mapping = find_game_signature_metadata_mapping(&game, conn).await?;

		if let Some(mapping) = mapping {
			if mapping.match_type != MatchTypeEnum::Failed
				|| mapping.match_type != MatchTypeEnum::None
			{
				debug!("Overwriting existing mapping for game: {}", game.id);
				// TODO: decide how to notify the user that this entry is already matched
			}
		}

		let updated = create_or_update_signature_metadata_mapping(
			SignatureMetadataMappingInputBuilder::default()
				.game_id(Some(game.id))
				.provider(r#match.provider.into())
				.provider_id(Some(r#match.provider_id.clone()))
				.match_type(MatchTypeEnum::Manual)
				.manual_match_type(Some(r#match.manual_match_type.into()))
				.failed_match_reason(None)
				.automatic_match_reason(None)
				.comment(r#match.comment.clone())
				.build()?,
			conn,
		)
		.await?;

		results.push(
			UpdatedMatchResultBuilder::default()
				.id(game.id)
				.external_metadata(vec![updated.into()])
				.build()?,
		)
	}

	Ok(results)
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
