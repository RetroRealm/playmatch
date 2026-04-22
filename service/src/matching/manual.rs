use crate::cache::CacheKey;
use crate::db::company::{find_company_by_name, find_company_related_signature_metadata_mapping};
use crate::db::game::{
	find_all_children_of_game, find_game_and_id_mapping_by_md5, find_game_and_id_mapping_by_sha1,
	find_game_and_id_mapping_by_sha256, find_game_by_name_or_game_file_name, find_game_parent,
	find_game_signature_metadata_mapping, find_games_by_name_and_platform_id,
};
use crate::db::game_file::get_game_files_from_game_ids;
use crate::db::platform::{
	find_platform_by_name, find_platform_of_game, find_platform_related_signature_metadata_mapping,
};
use crate::db::signature_metadata_mapping::{
	SignatureMetadataMappingInputBuilder, create_or_update_signature_metadata_mapping,
};
use crate::error::{ServiceError, ServiceResult};
use crate::identification::cache::{IdentifyCacheType, filename_size_key};
use crate::model::matching::{CompanyOrPlatformMatchRequest, GameMatchData, GameMatchRequest};
use crate::model::{
	GameMatchType, GameMetadataMatchResult, GameMetadataMatchResultBuilder, UpdatedMatchResult,
	UpdatedMatchResultBuilder,
};
use entity::game_file;
use entity::sea_orm_active_enums::{MatchTypeEnum, MetadataProviderEnum};
use entity::{game, signature_metadata_mapping};
use log::{debug, warn};
use redis::AsyncTypedCommands;
use redis::aio::MultiplexedConnection;
use sea_orm::DbConn;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
enum ManualTarget {
	Company,
	Platform,
}

impl ManualTarget {
	fn label(self) -> &'static str {
		match self {
			Self::Company => "company",
			Self::Platform => "platform",
		}
	}
}

pub async fn apply_manual_company_match(
	r#match: CompanyOrPlatformMatchRequest,
	conn: &DbConn,
) -> ServiceResult<UpdatedMatchResult> {
	apply_manual_entity_match(ManualTarget::Company, r#match, conn).await
}

pub async fn apply_manual_platform_match(
	r#match: CompanyOrPlatformMatchRequest,
	conn: &DbConn,
) -> ServiceResult<UpdatedMatchResult> {
	apply_manual_entity_match(ManualTarget::Platform, r#match, conn).await
}

async fn apply_manual_entity_match(
	target: ManualTarget,
	r#match: CompanyOrPlatformMatchRequest,
	conn: &DbConn,
) -> ServiceResult<UpdatedMatchResult> {
	let provider_enum: MetadataProviderEnum = r#match.provider.into();
	let (entity_id, existing_mapping) = match target {
		ManualTarget::Company => {
			let company = find_company_by_name(r#match.name.as_str(), conn)
				.await?
				.ok_or(ServiceError::CompanyNotFound)?;
			let mapping = find_company_related_signature_metadata_mapping(
				&company,
				provider_enum,
				conn,
			)
			.await?;
			(company.id, mapping)
		}
		ManualTarget::Platform => {
			let platform = find_platform_by_name(r#match.name.as_str(), conn)
				.await?
				.ok_or(ServiceError::PlatformNotFound)?;
			let mapping = find_platform_related_signature_metadata_mapping(
				&platform,
				provider_enum,
				conn,
			)
			.await?;
			(platform.id, mapping)
		}
	};

	if let Some(mapping) = existing_mapping {
		if mapping.match_type != MatchTypeEnum::Failed || mapping.match_type != MatchTypeEnum::None
		{
			debug!(
				"Overwriting existing mapping for {}: {entity_id}",
				target.label()
			);
			// TODO: decide how to notify the user that this entry is already matched
		}

		if let Some(provider_id) = &mapping.provider_id
			&& provider_id == &r#match.provider_id
			&& mapping.provider == r#match.provider.into()
		{
			debug!("No update needed for {}: {entity_id}", target.label());
			return Ok(UpdatedMatchResultBuilder::default()
				.id(entity_id)
				.external_metadata(mapping.into())
				.build()?);
		}
	}

	let mut input_builder = SignatureMetadataMappingInputBuilder::default();
	match target {
		ManualTarget::Company => {
			input_builder.company_id(Some(entity_id));
		}
		ManualTarget::Platform => {
			input_builder.platform_id(Some(entity_id));
		}
	}

	let updated = create_or_update_signature_metadata_mapping(
		input_builder
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
	crate::metrics::record_user_action(target.label(), "manual_match");

	Ok(UpdatedMatchResultBuilder::default()
		.id(entity_id)
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

	// Batch-fetch and index by game_id so the update loop is memory-only.
	let game_ids: Vec<_> = games_to_update.iter().map(|g| g.id).collect();
	let all_files = get_game_files_from_game_ids(&game_ids, db_conn).await?;
	let mut files_by_game: HashMap<_, Vec<game_file::Model>> = HashMap::new();
	for file in all_files {
		files_by_game.entry(file.game_id).or_default().push(file);
	}

	let mut results = vec![];
	let mut cache_keys_to_bust: Vec<String> = Vec::new();

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

		// Keys flushed as one pipelined DEL after the loop.
		if let Some(files) = files_by_game.get(&game.id) {
			for file in files {
				collect_identify_cache_keys(file, &mut cache_keys_to_bust);
			}
		}

		results.push(
			UpdatedMatchResultBuilder::default()
				.id(game.id)
				.external_metadata(updated.into())
				.build()?,
		)
	}

	if !cache_keys_to_bust.is_empty() {
		debug!(
			"Busting {} identify-cache keys in one DEL",
			cache_keys_to_bust.len()
		);
		if let Err(e) = redis_conn.del(&cache_keys_to_bust).await {
			warn!(
				"batch identify-cache delete failed for {} keys: {e}",
				cache_keys_to_bust.len()
			);
		}
	}

	debug!("Updated {} games", results.len());

	Ok(results)
}

fn collect_identify_cache_keys(file: &game_file::Model, out: &mut Vec<String>) {
	if let Some(sha256) = &file.sha256 {
		out.push(IdentifyCacheType::IdentifySha256.get_cache_key(sha256));
	}
	if let Some(sha1) = &file.sha1 {
		out.push(IdentifyCacheType::IdentifySha1.get_cache_key(sha1));
	}
	if let Some(md5) = &file.md5 {
		out.push(IdentifyCacheType::IdentifyMd5.get_cache_key(md5));
	}
	if let Some(size) = file.file_size_in_bytes {
		let key = filename_size_key(&file.file_name, size);
		out.push(IdentifyCacheType::IdentifyFilenameSize.get_cache_key(&key));
	}
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
