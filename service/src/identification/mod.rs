pub mod cache;
mod protocol;

use crate::cache::CacheStatus;
use crate::cache::CacheStatus::{Cached, NonCached};
use crate::db::game::{find_all_relations_of_game, get_game_by_id};
use crate::error::{ServiceError, ServiceResult};
use crate::identification::cache::{
	IdentifyEntry, find_game_and_metadata_ids_by_filename_size_cached,
	find_game_and_metadata_ids_by_hash_cached,
};
use crate::identification::protocol::IdentifyAggregator;
use crate::matching::manual::build_result;
use crate::model::{
	GameAndRelationMatchResult, GameAndRelationMatchResultBuilder, GameAndRelationsResult,
	GameAndRelationsResultBuilder, GameFileMatchSearch, GameMatchType, GameMetadataMatchResult,
	PlaymatchGame,
};
use log::debug;
use redis::aio::MultiplexedConnection;
use sea_orm::DbConn;
use sea_orm::prelude::Uuid;
use std::ops::ControlFlow;
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

/// Tries each match type in order (sha256, sha1, md5, name+size) and returns
/// the first hit. When every attempted hash produced a cached-but-empty
/// result the outcome collapses to `Cached(None)`, otherwise `NonCached(None)`.
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

	let mut agg = IdentifyAggregator::new(expected_count);

	for match_type in GameMatchType::iter().filter(|t| *t != GameMatchType::NoMatch) {
		let outcome = match match_type {
			GameMatchType::SHA256 => match &search.sha256 {
				Some(hash) => {
					find_game_and_metadata_ids_by_hash_cached(
						hash,
						GameMatchType::SHA256,
						redis_conn,
						db_conn,
					)
					.await?
				}
				None => continue,
			},
			GameMatchType::SHA1 => match &search.sha1 {
				Some(hash) => {
					find_game_and_metadata_ids_by_hash_cached(
						hash,
						GameMatchType::SHA1,
						redis_conn,
						db_conn,
					)
					.await?
				}
				None => continue,
			},
			GameMatchType::MD5 => match &search.md5 {
				Some(hash) => {
					find_game_and_metadata_ids_by_hash_cached(
						hash,
						GameMatchType::MD5,
						redis_conn,
						db_conn,
					)
					.await?
				}
				None => continue,
			},
			GameMatchType::FileNameAndSize => {
				find_game_and_metadata_ids_by_filename_size_cached(
					&search.file_name,
					search.file_size,
					redis_conn,
					db_conn,
				)
				.await?
			}
			GameMatchType::NoMatch => unreachable!(),
		};

		let hit = matches!(&outcome, Cached(Some(_)) | NonCached(Some(_)));
		crate::metrics::record_identify_attempt(match_type.metric_label(), hit);

		if let ControlFlow::Break(result) = agg.observe(match_type, outcome) {
			debug!("Identify resolved on match type {match_type:?}");
			return Ok(result);
		}
	}

	let result = agg.finalize();
	if matches!(result, Cached(None)) {
		debug!("All possible game matches were cached as empty, returning cached no-match");
	}
	Ok(result)
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
