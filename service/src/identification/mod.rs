pub mod cache;

use crate::cache::CacheStatus;
use crate::cache::CacheStatus::{Cached, NonCached};
use crate::db::game::{
	find_all_relations_of_game, find_game_and_id_mapping_by_name_and_size, get_game_by_id,
};
use crate::error::{ServiceError, ServiceResult};
use crate::identification::cache::{
	IdentifyEntry, find_game_and_metadata_ids_by_md5_cached,
	find_game_and_metadata_ids_by_sha1_cached, find_game_and_metadata_ids_by_sha256_cached,
};
use crate::manual_match::build_result;
use crate::model::{
	GameAndRelationMatchResult, GameAndRelationMatchResultBuilder, GameAndRelationsResult,
	GameAndRelationsResultBuilder, GameFileMatchSearch, GameMatchType, GameMetadataMatchResult,
	PlaymatchGame,
};
use log::debug;
use redis::aio::MultiplexedConnection;
use sea_orm::DbConn;
use sea_orm::prelude::Uuid;
use strum::IntoEnumIterator;

pub async fn get_game_by_id_from_db(game_id: Uuid, conn: &DbConn) -> ServiceResult<PlaymatchGame> {
	let game_opt = get_game_by_id(game_id, conn).await?;

	let game = game_opt.ok_or(ServiceError::GameNotFound)?;

	Ok(game.into())
}

pub async fn get_game_and_all_relations(
	game_id: Uuid,
	conn: &DbConn,
) -> ServiceResult<GameAndRelationsResult> {
	let game_opt = get_game_by_id(game_id, conn).await?;

	let game = game_opt.ok_or(ServiceError::GameNotFound)?;

	let (dat_file_import, dat_file, signature_group, platform, company, game_files) =
		find_all_relations_of_game(&game, conn).await?;

	Ok(GameAndRelationsResultBuilder::default()
		.game(game.into())
		.platform(platform.into())
		.company(company.map(|c| c.into()))
		.game_files(game_files.into_iter().map(|gf| gf.into()).collect())
		.dat_file(dat_file.into())
		.dat_file_import(dat_file_import.into())
		.signature_group(signature_group.into())
		.build()?)
}

pub async fn identify_game_and_get_relations(
	search: GameFileMatchSearch,
	redis_conn: &mut MultiplexedConnection,
	db_conn: &DbConn,
) -> anyhow::Result<CacheStatus<GameAndRelationMatchResult>> {
	let outcome = identify_game(&search, redis_conn, db_conn).await?;

	match outcome {
		Cached(Some((match_type, entry))) => Ok(Cached(
			build_relation_match_result(match_type, entry, db_conn).await?,
		)),
		NonCached(Some((match_type, entry))) => Ok(NonCached(
			build_relation_match_result(match_type, entry, db_conn).await?,
		)),
		Cached(None) => Ok(Cached(empty_relation_match_result())),
		NonCached(None) => Ok(NonCached(empty_relation_match_result())),
	}
}

pub async fn identify_game_and_metadata_mappings(
	search: GameFileMatchSearch,
	redis_conn: &mut MultiplexedConnection,
	db_conn: &DbConn,
) -> anyhow::Result<CacheStatus<GameMetadataMatchResult>> {
	let outcome = identify_game(&search, redis_conn, db_conn).await?;

	match outcome {
		Cached(Some((match_type, entry))) => Ok(Cached(build_result(
			match_type,
			entry.game,
			entry.metadata_mappings,
		)?)),
		NonCached(Some((match_type, entry))) => Ok(NonCached(build_result(
			match_type,
			entry.game,
			entry.metadata_mappings,
		)?)),
		Cached(None) => Ok(Cached(empty_metadata_match_result())),
		NonCached(None) => Ok(NonCached(empty_metadata_match_result())),
	}
}

/// Walk the supported match types in order (sha256, sha1, md5, name+size) and return the first
/// hit, together with whether the hit was served from cache. If every hash that could have been
/// checked produced a cached-but-empty result, surface a `Cached(None)`; otherwise `NonCached(None)`.
async fn identify_game(
	search: &GameFileMatchSearch,
	redis_conn: &mut MultiplexedConnection,
	db_conn: &DbConn,
) -> anyhow::Result<CacheStatus<Option<(GameMatchType, IdentifyEntry)>>> {
	let expected_count = [
		search.sha256.as_ref(),
		search.sha1.as_ref(),
		search.md5.as_ref(),
	]
	.iter()
	.filter(|hash| hash.is_some())
	.count();

	let mut cached_but_empty = 0;

	for match_type in GameMatchType::iter().filter(|t| *t != GameMatchType::NoMatch) {
		let type_result = match match_type {
			GameMatchType::SHA256 => match &search.sha256 {
				Some(hash) => {
					find_game_and_metadata_ids_by_sha256_cached(hash, redis_conn, db_conn).await?
				}
				None => NonCached(None),
			},
			GameMatchType::SHA1 => match &search.sha1 {
				Some(hash) => {
					find_game_and_metadata_ids_by_sha1_cached(hash, redis_conn, db_conn).await?
				}
				None => NonCached(None),
			},
			GameMatchType::MD5 => match &search.md5 {
				Some(hash) => {
					find_game_and_metadata_ids_by_md5_cached(hash, redis_conn, db_conn).await?
				}
				None => NonCached(None),
			},
			GameMatchType::FileNameAndSize => NonCached(
				find_game_and_id_mapping_by_name_and_size(
					&search.file_name,
					search.file_size,
					db_conn,
				)
				.await?
				.map(|(game, metadata_mappings)| IdentifyEntry {
					game,
					metadata_mappings,
				}),
			),
			GameMatchType::NoMatch => unreachable!(),
		};

		match type_result {
			Cached(Some(entry)) => {
				debug!("Cache hit for game match ({match_type:?}): {entry:?}");
				return Ok(Cached(Some((match_type, entry))));
			}
			NonCached(Some(entry)) => {
				debug!("Cache miss for game match ({match_type:?}): {entry:?}");
				return Ok(NonCached(Some((match_type, entry))));
			}
			Cached(None) => cached_but_empty += 1,
			NonCached(None) => continue,
		}
	}

	if cached_but_empty == expected_count && cached_but_empty != 0 {
		debug!("All possible game matches were cached as empty, returning cached no-match");
		Ok(Cached(None))
	} else {
		Ok(NonCached(None))
	}
}

async fn build_relation_match_result(
	match_type: GameMatchType,
	entry: IdentifyEntry,
	db_conn: &DbConn,
) -> anyhow::Result<GameAndRelationMatchResult> {
	let (dat_file_import, dat_file, signature_group, platform, company, game_files) =
		find_all_relations_of_game(&entry.game, db_conn).await?;

	Ok(GameAndRelationMatchResultBuilder::default()
		.game_match_type(match_type)
		.game(Some(entry.game.into()))
		.platform(Some(platform.into()))
		.company(company.map(|c| c.into()))
		.game_files(game_files.into_iter().map(|gf| gf.into()).collect())
		.dat_file(Some(dat_file.into()))
		.dat_file_import(Some(dat_file_import.into()))
		.signature_group(Some(signature_group.into()))
		.external_metadata(
			entry
				.metadata_mappings
				.into_iter()
				.map(|m| m.into())
				.collect(),
		)
		.build()?)
}

fn empty_relation_match_result() -> GameAndRelationMatchResult {
	GameAndRelationMatchResult {
		game_match_type: GameMatchType::NoMatch,
		game: None,
		game_files: vec![],
		company: None,
		platform: None,
		dat_file_import: None,
		dat_file: None,
		signature_group: None,
		external_metadata: vec![],
	}
}

fn empty_metadata_match_result() -> GameMetadataMatchResult {
	GameMetadataMatchResult {
		game_match_type: GameMatchType::NoMatch,
		id: None,
		external_metadata: Vec::new(),
	}
}
