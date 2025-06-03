use crate::cache::identify::{
	find_game_and_id_mapping_by_md5_cached, find_game_and_id_mapping_by_sha1_cached,
	find_game_and_id_mapping_by_sha256_cached,
};
use crate::db::game::{
	find_all_children_of_game, find_game_and_id_mapping_by_md5,
	find_game_and_id_mapping_by_name_and_size, find_game_and_id_mapping_by_sha1,
	find_game_and_id_mapping_by_sha256, find_game_by_name_or_game_file_name, find_game_parent,
	find_game_signature_metadata_mapping, find_games_by_name_and_platform_id,
};
use crate::db::game_file::get_game_files_from_game_id;
use crate::db::platform::find_platform_of_game;
use crate::db::signature_metadata_mapping::{
	create_or_update_signature_metadata_mapping, SignatureMetadataMappingInputBuilder,
};
use crate::error::{ServiceError, ServiceResult};
use crate::model::{
	GameFileMatchSearch, GameMatchResult, GameMatchResultBuilder, GameMatchType, MatchRequest,
	UpdatedMatchResult, UpdatedMatchResultBuilder,
};
use cached::Cached;
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
			.map(|(game, _)| game)
	} else if let Some(sha1) = &r#match.sha1 {
		find_game_and_id_mapping_by_sha1(sha1, conn)
			.await?
			.map(|(game, _)| game)
	} else if let Some(md5) = &r#match.md5 {
		find_game_and_id_mapping_by_md5(md5, conn)
			.await?
			.map(|(game, _)| game)
	} else if let Some(file_name) = &r#match.name {
		find_game_by_name_or_game_file_name(file_name, conn).await?
	} else {
		None
	};

	let game = found_game.ok_or(ServiceError::GameNotFound)?;
	let platform = find_platform_of_game(game.id, conn).await?;

	// Find all games that match the name and have the same platform (this is useful for platforms having multiple dat sets for encrypted and decrypted versions)
	let games = if let Some(platform) = platform {
		find_games_by_name_and_platform_id(&game.name, platform.id, conn).await?
	} else {
		vec![game]
	};

	let mut games_to_update = vec![];

	// Find all parents and children of the same game
	for game in games {
		if let Some(parent) = find_game_parent(&game, conn).await? {
			debug!("Found parent game: {}", parent.id);

			let children = find_all_children_of_game(&parent, conn).await?;
			debug!(
				"Found {} children for parent game: {}",
				children.len(),
				parent.id
			);

			games_to_update.push(parent);
			games_to_update.extend(children);
		} else {
			debug!("No parent game found for game: {}", game.id);
			let children = find_all_children_of_game(&game, conn).await?;
			debug!("Found {} children for game: {}", children.len(), game.id);
			games_to_update.push(game);
			games_to_update.extend(children);
		}
	}

	let mut results = vec![];

	for game in games_to_update {
		debug!("Updating game: {}", game.name);

		let mapping = find_game_signature_metadata_mapping(&game, conn).await?;

		if let Some(mapping) = mapping {
			if mapping.match_type != MatchTypeEnum::Failed
				|| mapping.match_type != MatchTypeEnum::None
			{
				debug!("Overwriting existing mapping for game: {}", game.id);
				// TODO: decide how to notify the user that this entry is already matched
			}

			if let Some(provider_id) = &mapping.provider_id {
				if provider_id == &r#match.provider_id
					&& mapping.provider == r#match.provider.into()
				{
					debug!("No update needed for game: {}", game.id);
					continue;
				}
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

		let mut sha256_lock = crate::cache::identify::FIND_GAME_AND_ID_MAPPING_BY_SHA256_CACHED
			.lock()
			.await;
		let mut sha1_lock = crate::cache::identify::FIND_GAME_AND_ID_MAPPING_BY_SHA1_CACHED
			.lock()
			.await;
		let mut md5_lock = crate::cache::identify::FIND_GAME_AND_ID_MAPPING_BY_MD5_CACHED
			.lock()
			.await;

		let game_files = get_game_files_from_game_id(game.id, conn).await?;

		for game_file in game_files {
			if let Some(sha256) = &game_file.sha256 {
				sha256_lock.cache_remove(sha256);
			}
			if let Some(sha1) = &game_file.sha1 {
				sha1_lock.cache_remove(sha1);
			}
			if let Some(md5) = &game_file.md5 {
				md5_lock.cache_remove(md5);
			}
		}

		results.push(
			UpdatedMatchResultBuilder::default()
				.id(game.id)
				.external_metadata(vec![updated.into()])
				.build()?,
		)
	}

	debug!("Updated {} games", results.len());

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
