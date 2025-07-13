use crate::db::company::{find_company_by_name, find_company_related_signature_metadata_mapping};
use crate::db::game::{
	find_all_children_of_game, find_game_and_id_mapping_by_md5, find_game_and_id_mapping_by_sha1,
	find_game_and_id_mapping_by_sha256, find_game_by_name_or_game_file_name, find_game_parent,
	find_game_signature_metadata_mapping, find_games_by_name_and_platform_id,
};
use crate::db::game_file::get_game_files_from_game_id;
use crate::db::platform::{
	find_platform_by_name, find_platform_of_game, find_platform_related_signature_metadata_mapping,
};
use crate::db::signature_metadata_mapping::{
	SignatureMetadataMappingInputBuilder, create_or_update_signature_metadata_mapping,
};
use crate::error::{ServiceError, ServiceResult};
use crate::model::matching::{CompanyMatchRequest, GameMatchRequest, PlatformMatchRequest};
use crate::model::{
	GameMatchResult, GameMatchResultBuilder, GameMatchType, UpdatedMatchResult,
	UpdatedMatchResultBuilder,
};
use cached::Cached;
use entity::sea_orm_active_enums::MatchTypeEnum;
use entity::{game, signature_metadata_mapping};
use log::debug;
use sea_orm::DbConn;

pub async fn apply_manual_company_match(
	r#match: CompanyMatchRequest,
	user: entity::user::Model,
	conn: &DbConn,
) -> ServiceResult<UpdatedMatchResult> {
	let found_company = find_company_by_name(r#match.name.as_str(), conn).await?;

	let company = found_company.ok_or(ServiceError::CompanyNotFound)?;

	let mapping = find_company_related_signature_metadata_mapping(&company, conn).await?;

	if let Some(mapping) = mapping {
		if mapping.match_type != MatchTypeEnum::Failed || mapping.match_type != MatchTypeEnum::None
		{
			debug!("Overwriting existing mapping for company: {}", company.id);
			// TODO: decide how to notify the user that this entry is already matched
		}

		if let Some(provider_id) = &mapping.provider_id {
			if provider_id == &r#match.provider_id && mapping.provider == r#match.provider.into() {
				debug!("No update needed for company: {}", company.id);
				return Ok(UpdatedMatchResultBuilder::default()
					.id(company.id)
					.external_metadata(mapping.into())
					.build()?);
			}
		}
	}

	let updated = create_or_update_signature_metadata_mapping(
		SignatureMetadataMappingInputBuilder::default()
			.company_id(Some(company.id))
			.provider(r#match.provider.into())
			.provider_id(Some(r#match.provider_id.clone()))
			.match_type(MatchTypeEnum::Manual)
			.manual_match_type(Some(r#match.manual_match_type.into()))
			.failed_match_reason(None)
			.automatic_match_reason(None)
			.comment(r#match.comment.clone())
			.manually_matched_by(Some(user.id))
			.build()?,
		conn,
	)
	.await?;

	Ok(UpdatedMatchResultBuilder::default()
		.id(company.id)
		.external_metadata(updated.into())
		.build()?)
}

pub async fn apply_manual_platform_match(
	r#match: PlatformMatchRequest,
	user: entity::user::Model,
	conn: &DbConn,
) -> ServiceResult<UpdatedMatchResult> {
	let found_platform = find_platform_by_name(r#match.name.as_str(), conn).await?;

	let platform = found_platform.ok_or(ServiceError::PlatformNotFound)?;

	let mapping = find_platform_related_signature_metadata_mapping(&platform, conn).await?;

	if let Some(mapping) = mapping {
		if mapping.match_type != MatchTypeEnum::Failed || mapping.match_type != MatchTypeEnum::None
		{
			debug!("Overwriting existing mapping for platform: {}", platform.id);
			// TODO: decide how to notify the user that this entry is already matched
		}

		if let Some(provider_id) = &mapping.provider_id {
			if provider_id == &r#match.provider_id && mapping.provider == r#match.provider.into() {
				debug!("No update needed for platform: {}", platform.id);
				return Ok(UpdatedMatchResultBuilder::default()
					.id(platform.id)
					.external_metadata(mapping.into())
					.build()?);
			}
		}
	}

	let updated = create_or_update_signature_metadata_mapping(
		SignatureMetadataMappingInputBuilder::default()
			.platform_id(Some(platform.id))
			.provider(r#match.provider.into())
			.provider_id(Some(r#match.provider_id.clone()))
			.match_type(MatchTypeEnum::Manual)
			.manual_match_type(Some(r#match.manual_match_type.into()))
			.failed_match_reason(None)
			.automatic_match_reason(None)
			.comment(r#match.comment.clone())
			.manually_matched_by(Some(user.id))
			.build()?,
		conn,
	)
	.await?;

	Ok(UpdatedMatchResultBuilder::default()
		.id(platform.id)
		.external_metadata(updated.into())
		.build()?)
}

pub async fn apply_manual_game_match(
	r#match: GameMatchRequest,
	user: entity::user::Model,
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
				.manually_matched_by(Some(user.id))
				.build()?,
			conn,
		)
		.await?;

		// Bust the cache for the hashes of the game files associated with this game so that the next time it is queried, it will return the updated mapping
		let game_files = get_game_files_from_game_id(game.id, conn).await?;
		for game_file in game_files {
			bust_cache_for_hashes(game_file.sha256, game_file.sha1, game_file.md5).await
		}

		results.push(
			UpdatedMatchResultBuilder::default()
				.id(game.id)
				.external_metadata(updated.into())
				.build()?,
		)
	}

	debug!("Updated {} games", results.len());

	Ok(results)
}

async fn bust_cache_for_hashes(sha256: Option<String>, sha1: Option<String>, md5: Option<String>) {
	let mut sha256_lock = crate::cache::identify::FIND_GAME_AND_ID_MAPPING_BY_SHA256_CACHED
		.lock()
		.await;
	let mut sha1_lock = crate::cache::identify::FIND_GAME_AND_ID_MAPPING_BY_SHA1_CACHED
		.lock()
		.await;
	let mut md5_lock = crate::cache::identify::FIND_GAME_AND_ID_MAPPING_BY_MD5_CACHED
		.lock()
		.await;

	if let Some(sha256) = &sha256 {
		sha256_lock.cache_remove(sha256);
	}
	if let Some(sha1) = &sha1 {
		sha1_lock.cache_remove(sha1);
	}
	if let Some(md5) = &md5 {
		md5_lock.cache_remove(md5);
	}
}

pub(crate) fn build_result(
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
