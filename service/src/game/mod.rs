use crate::cache::CacheStatus;
use crate::cache::identify::{
	IdentifyEntry, find_game_and_metadata_ids_by_md5_cached,
	find_game_and_metadata_ids_by_sha1_cached, find_game_and_metadata_ids_by_sha256_cached,
};
use crate::db::game::{
	find_all_relations_of_game, find_game_and_id_mapping_by_name_and_size, get_game_by_id,
};
use crate::error::{ServiceError, ServiceResult};
use crate::manual_match::build_result;
use crate::model::{
	GameAndRelationMatchResult, GameAndRelationMatchResultBuilder, GameAndRelationsResult,
	GameAndRelationsResultBuilder, GameFileMatchSearch, GameMatchType, GameMetadataMatchResult,
	PlaymatchGame,
};
use CacheStatus::{Cached, NonCached};
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
	let expected_count = [
		search.sha256.as_ref(),
		search.sha1.as_ref(),
		search.md5.as_ref(),
	]
	.iter()
	.filter(|hash| hash.is_some())
	.count();

	let mut cached_results = 0;

	for r#type in GameMatchType::iter().filter(|t| *t != GameMatchType::NoMatch) {
		let type_result = match r#type {
			GameMatchType::SHA256 => {
				if let Some(sha256) = &search.sha256 {
					find_game_and_metadata_ids_by_sha256_cached(sha256, redis_conn, db_conn).await?
				} else {
					NonCached(None)
				}
			}
			GameMatchType::SHA1 => {
				if let Some(sha1) = &search.sha1 {
					find_game_and_metadata_ids_by_sha1_cached(sha1, redis_conn, db_conn).await?
				} else {
					NonCached(None)
				}
			}
			GameMatchType::MD5 => {
				if let Some(md5) = &search.md5 {
					find_game_and_metadata_ids_by_md5_cached(md5, redis_conn, db_conn).await?
				} else {
					NonCached(None)
				}
			}
			GameMatchType::FileNameAndSize => NonCached(
				find_game_and_id_mapping_by_name_and_size(
					&search.file_name,
					search.file_size,
					db_conn,
				)
				.await?
				.map(|r| IdentifyEntry {
					game: r.0,
					metadata_mappings: r.1,
				}),
			),
			GameMatchType::NoMatch => unreachable!(),
		};

		match type_result {
			Cached(Some(entry)) => {
				debug!("Cache hit for game and relations match: {entry:?}");

				let (dat_file_import, dat_file, signature_group, platform, company, game_files) =
					find_all_relations_of_game(&entry.game, db_conn).await?;

				return Ok(Cached(
					GameAndRelationMatchResultBuilder::default()
						.game_match_type(r#type)
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
						.build()?,
				));
			}
			NonCached(Some(entry)) => {
				debug!("Cache miss for game and relations match: {entry:?}");

				let (dat_file_import, dat_file, signature_group, platform, company, game_files) =
					find_all_relations_of_game(&entry.game, db_conn).await?;

				return Ok(Cached(
					GameAndRelationMatchResultBuilder::default()
						.game_match_type(r#type)
						.game(Some(entry.game.into()))
						.platform(Some(platform.into()))
						.company(company.map(|c| c.into()))
						.game_files(game_files.into_iter().map(|gf| gf.into()).collect())
						.dat_file(Some(dat_file.into()))
						.dat_file_import(Some(dat_file_import.into()))
						.signature_group(Some(signature_group.into()))
						.build()?,
				));
			}
			Cached(None) => {
				cached_results += 1;
			}
			_ => continue,
		}
	}

	if cached_results == expected_count {
		debug!("All (possible) game metadata matches were cached, returning cached result");
		return Ok(Cached(GameAndRelationMatchResult {
			game_match_type: GameMatchType::NoMatch,
			game: None,
			game_files: vec![],
			company: None,
			platform: None,
			dat_file_import: None,
			dat_file: None,
			signature_group: None,
			external_metadata: vec![],
		}));
	}

	Ok(NonCached(GameAndRelationMatchResult {
		game_match_type: GameMatchType::NoMatch,
		game: None,
		game_files: vec![],
		company: None,
		platform: None,
		dat_file_import: None,
		dat_file: None,
		signature_group: None,
		external_metadata: vec![],
	}))
}

pub async fn identify_game_and_metadata_mappings(
	search: GameFileMatchSearch,
	redis_conn: &mut MultiplexedConnection,
	db_conn: &DbConn,
) -> anyhow::Result<CacheStatus<GameMetadataMatchResult>> {
	let expected_count = [
		search.sha256.as_ref(),
		search.sha1.as_ref(),
		search.md5.as_ref(),
	]
	.iter()
	.filter(|hash| hash.is_some())
	.count();

	let mut cached_results = 0;

	for r#type in GameMatchType::iter().filter(|t| *t != GameMatchType::NoMatch) {
		let type_result = match r#type {
			GameMatchType::SHA256 => {
				if let Some(sha256) = &search.sha256 {
					find_game_and_metadata_ids_by_sha256_cached(sha256, redis_conn, db_conn).await?
				} else {
					NonCached(None)
				}
			}
			GameMatchType::SHA1 => {
				if let Some(sha1) = &search.sha1 {
					find_game_and_metadata_ids_by_sha1_cached(sha1, redis_conn, db_conn).await?
				} else {
					NonCached(None)
				}
			}
			GameMatchType::MD5 => {
				if let Some(md5) = &search.md5 {
					find_game_and_metadata_ids_by_md5_cached(md5, redis_conn, db_conn).await?
				} else {
					NonCached(None)
				}
			}
			GameMatchType::FileNameAndSize => NonCached(
				find_game_and_id_mapping_by_name_and_size(
					&search.file_name,
					search.file_size,
					db_conn,
				)
				.await?
				.map(|r| IdentifyEntry {
					game: r.0,
					metadata_mappings: r.1,
				}),
			),
			GameMatchType::NoMatch => unreachable!(),
		};

		match type_result {
			Cached(Some(entry)) => {
				debug!("Cache hit for game metadata match: {entry:?}");
				return Ok(Cached(build_result(
					r#type,
					entry.game,
					entry.metadata_mappings,
				)?));
			}
			NonCached(Some(entry)) => {
				debug!("Cache miss for game metadata match: {entry:?}");
				return Ok(NonCached(build_result(
					r#type,
					entry.game,
					entry.metadata_mappings,
				)?));
			}
			Cached(None) => {
				cached_results += 1;
			}
			_ => continue,
		}
	}

	if cached_results == expected_count {
		debug!("All (possible) game metadata matches were cached, returning cached result");
		return Ok(Cached(GameMetadataMatchResult {
			game_match_type: GameMatchType::NoMatch,
			id: None,
			external_metadata: Vec::new(),
		}));
	}

	Ok(NonCached(GameMetadataMatchResult {
		game_match_type: GameMatchType::NoMatch,
		id: None,
		external_metadata: Vec::new(),
	}))
}
