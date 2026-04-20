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
use crate::identification::cache::{IdentifyCacheType, delete_identify_cache, filename_size_key};
use crate::model::matching::{CompanyOrPlatformMatchRequest, GameMatchData, GameMatchRequest};
use crate::model::{
	GameMatchType, GameMetadataMatchResult, GameMetadataMatchResultBuilder, UpdatedMatchResult,
	UpdatedMatchResultBuilder,
};
use entity::sea_orm_active_enums::MatchTypeEnum;
use entity::{game, signature_metadata_mapping};
use log::debug;
use redis::aio::MultiplexedConnection;
use sea_orm::DbConn;

pub async fn apply_manual_company_match(
	r#match: CompanyOrPlatformMatchRequest,
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

		if let Some(provider_id) = &mapping.provider_id
			&& provider_id == &r#match.provider_id
			&& mapping.provider == r#match.provider.into()
		{
			debug!("No update needed for company: {}", company.id);
			return Ok(UpdatedMatchResultBuilder::default()
				.id(company.id)
				.external_metadata(mapping.into())
				.build()?);
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
			.manually_matched_by(r#match.user_id)
			.build()?,
		conn,
	)
	.await?;
	crate::metrics::record_user_action("company", "manual_match");

	Ok(UpdatedMatchResultBuilder::default()
		.id(company.id)
		.external_metadata(updated.into())
		.build()?)
}

pub async fn apply_manual_platform_match(
	r#match: CompanyOrPlatformMatchRequest,
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

		if let Some(provider_id) = &mapping.provider_id
			&& provider_id == &r#match.provider_id
			&& mapping.provider == r#match.provider.into()
		{
			debug!("No update needed for platform: {}", platform.id);
			return Ok(UpdatedMatchResultBuilder::default()
				.id(platform.id)
				.external_metadata(mapping.into())
				.build()?);
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
			.manually_matched_by(r#match.user_id)
			.build()?,
		conn,
	)
	.await?;
	crate::metrics::record_user_action("platform", "manual_match");

	Ok(UpdatedMatchResultBuilder::default()
		.id(platform.id)
		.external_metadata(updated.into())
		.build()?)
}

pub async fn apply_manual_game_match(
	r#match: GameMatchRequest,
	db_conn: &DbConn,
	redis_conn: &mut MultiplexedConnection,
) -> ServiceResult<Vec<UpdatedMatchResult>> {
	let found_game = if let Some(sha256) = &r#match.sha256 {
		find_game_and_id_mapping_by_sha256(sha256, db_conn)
			.await?
			.map(|(game, _)| game)
	} else if let Some(sha1) = &r#match.sha1 {
		find_game_and_id_mapping_by_sha1(sha1, db_conn)
			.await?
			.map(|(game, _)| game)
	} else if let Some(md5) = &r#match.md5 {
		find_game_and_id_mapping_by_md5(md5, db_conn)
			.await?
			.map(|(game, _)| game)
	} else if let Some(file_name) = &r#match.name {
		find_game_by_name_or_game_file_name(file_name, db_conn).await?
	} else {
		None
	};

	let game = found_game.ok_or(ServiceError::GameNotFound)?;

	let results =
		apply_manual_game_match_by_game(game, r#match.into(), db_conn, redis_conn).await?;
	crate::metrics::record_user_action("game", "manual_match");
	Ok(results)
}

pub async fn apply_manual_game_match_by_game(
	game: game::Model,
	r#match: GameMatchData,
	db_conn: &DbConn,
	redis_conn: &mut MultiplexedConnection,
) -> ServiceResult<Vec<UpdatedMatchResult>> {
	let platform = find_platform_of_game(game.id, db_conn).await?;

	// Find all games that match the name and have the same platform (this is useful for platforms having multiple dat sets for encrypted and decrypted versions)
	let games = if let Some(platform) = platform {
		find_games_by_name_and_platform_id(&game.name, platform.id, db_conn).await?
	} else {
		vec![game]
	};

	let mut games_to_update = vec![];

	// Find all parents and children of the same game
	for game in games {
		if let Some(parent) = find_game_parent(&game, db_conn).await? {
			debug!("Found parent game: {}", parent.id);

			let children = find_all_children_of_game(&parent, db_conn).await?;
			debug!(
				"Found {} children for parent game: {}",
				children.len(),
				parent.id
			);

			games_to_update.push(parent);
			games_to_update.extend(children);
		} else {
			debug!("No parent game found for game: {}", game.id);
			let children = find_all_children_of_game(&game, db_conn).await?;
			debug!("Found {} children for game: {}", children.len(), game.id);
			games_to_update.push(game);
			games_to_update.extend(children);
		}
	}

	let mut results = vec![];

	for game in games_to_update {
		debug!("Updating game: {}", game.name);

		let mapping = find_game_signature_metadata_mapping(&game, db_conn).await?;

		if let Some(mapping) = mapping {
			if mapping.match_type != MatchTypeEnum::Failed
				|| mapping.match_type != MatchTypeEnum::None
			{
				debug!("Overwriting existing mapping for game: {}", game.id);
				// TODO: decide how to notify the user that this entry is already matched
			}

			if let Some(provider_id) = &mapping.provider_id
				&& provider_id == &r#match.provider_id
				&& mapping.provider == r#match.provider.into()
			{
				debug!("No update needed for game: {}", game.id);
				continue;
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
				.manually_matched_by(r#match.user_id)
				.build()?,
			db_conn,
		)
		.await?;

		// Bust the cache for the hashes of the game files associated with this game so that the next time it is queried, it will return the updated mapping
		let game_files = get_game_files_from_game_id(game.id, db_conn).await?;
		for game_file in game_files {
			bust_cache_for_hashes(
				game_file.sha256,
				game_file.sha1,
				game_file.md5,
				game_file.file_name,
				game_file.file_size_in_bytes,
				redis_conn,
			)
			.await?
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

async fn bust_cache_for_hashes(
	sha256: Option<String>,
	sha1: Option<String>,
	md5: Option<String>,
	file_name: String,
	file_size: Option<i64>,
	redis_conn: &mut MultiplexedConnection,
) -> ServiceResult<()> {
	if let Some(sha256) = &sha256 {
		delete_identify_cache(sha256, IdentifyCacheType::IdentifySha256, redis_conn).await?;
	}
	if let Some(sha1) = &sha1 {
		delete_identify_cache(sha1, IdentifyCacheType::IdentifySha1, redis_conn).await?;
	}
	if let Some(md5) = &md5 {
		delete_identify_cache(md5, IdentifyCacheType::IdentifyMd5, redis_conn).await?;
	}
	if let Some(size) = file_size {
		let key = filename_size_key(&file_name, size);
		delete_identify_cache(&key, IdentifyCacheType::IdentifyFilenameSize, redis_conn).await?;
	}
	debug!("Cache busted for hashes: sha256: {sha256:?}, sha1: {sha1:?}, md5: {md5:?}");

	Ok(())
}

pub fn build_result(
	game_match_type: GameMatchType,
	game: game::Model,
	signature_metadata_mappings: Vec<signature_metadata_mapping::Model>,
) -> anyhow::Result<GameMetadataMatchResult> {
	let result = GameMetadataMatchResultBuilder::default()
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
