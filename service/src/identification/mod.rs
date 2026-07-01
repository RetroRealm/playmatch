pub mod cache;
mod protocol;

use crate::cache::CacheStatus;
use crate::cache::CacheStatus::{Cached, NonCached};
use crate::db::game::{
	find_all_relations_of_game, find_all_signature_metadata_mappings_for_game, get_game_by_id,
};
use crate::error::{ServiceError, ServiceResult};
use crate::identification::cache::{
	IdentifyEntry, find_all_games_and_metadata_ids_by_crc_size_cached,
	find_all_games_and_metadata_ids_by_hash_cached, find_game_and_metadata_ids_by_crc_size_cached,
	find_game_and_metadata_ids_by_filename_size_cached, find_game_and_metadata_ids_by_hash_cached,
};
use crate::identification::protocol::IdentifyAggregator;
use crate::matching::manual::build_result;
use crate::model::{
	GameAndRelationMatchResult, GameAndRelationMatchResultBuilder, GameAndRelationMatchResultV2,
	GameAndRelationsResult, GameAndRelationsResultBuilder, GameAndRelationsResultV2,
	GameFileMatchSearch, GameMatchType, GameMetadataMatchResult, GameMetadataResponse,
	PlaymatchGame, PlaymatchGameFile,
};
use entity::{dat_file_import, game, game_file};
use log::debug;
use redis::aio::MultiplexedConnection;
use sea_orm::prelude::Uuid;
use sea_orm::{ColumnTrait, DbConn, EntityTrait, QueryFilter};
use std::collections::HashMap;
use std::ops::ControlFlow;
use strum::IntoEnumIterator;

pub async fn get_game_by_id_from_db(
	game_id: Uuid,
	conn: &DbConn,
) -> ServiceResult<GameMetadataResponse> {
	let game = get_game_by_id(game_id, conn)
		.await?
		.ok_or(ServiceError::GameNotFound)?;

	let mappings = find_all_signature_metadata_mappings_for_game(game.id, conn).await?;

	Ok(GameMetadataResponse {
		id: game.id,
		name: game.name,
		description: game.description,
		categories: game.categories,
		clone_of: game.clone_of,
		created_at: game.created_at.into(),
		updated_at: game.updated_at.into(),
		external_metadata: mappings.into_iter().map(Into::into).collect(),
	})
}

/// Return the dat file imports a game file's hash was seen in, newest first.
pub async fn get_game_file_history(
	game_file_id: Uuid,
	conn: &DbConn,
) -> ServiceResult<Vec<crate::model::PlaymatchDatFileImport>> {
	let imports = crate::db::game::get_game_file_presence_history(game_file_id, conn).await?;
	Ok(imports.into_iter().map(Into::into).collect())
}

pub async fn get_game_and_all_relations(
	game_id: Uuid,
	conn: &DbConn,
) -> ServiceResult<GameAndRelationsResult> {
	let game_opt = get_game_by_id(game_id, conn).await?;

	let game = game_opt.ok_or(ServiceError::GameNotFound)?;

	let (dat_file_import, dat_file, signature_group, platform, company, game_files) =
		find_all_relations_of_game(&game, conn).await?;

	let mappings = find_all_signature_metadata_mappings_for_game(game.id, conn).await?;

	let latest = dat_file.latest_dat_file_import_id;
	let versions =
		versions_by_last_seen(game.last_seen_dat_file_import_id, &game_files, conn).await?;

	Ok(GameAndRelationsResultBuilder::default()
		.game(enrich_game(game, latest, &versions))
		.platform(platform.into())
		.company(company.map(|c| c.into()))
		.game_files(
			game_files
				.into_iter()
				.map(|gf| enrich_game_file(gf, latest, &versions))
				.collect(),
		)
		.dat_file(dat_file.into())
		.dat_file_import(dat_file_import.into())
		.signature_group(signature_group.into())
		.external_metadata(mappings.into_iter().map(Into::into).collect())
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

/// V2 identify: the V1 primary stays element zero, and every co-hashed sibling
/// the ranked resolver returns after it is surfaced in `additionalMatches`.
///
/// The winning match type and primary game are taken from the shared
/// [`identify_game`] so the V2 primary is byte-identical to V1. Siblings are
/// then read from the ranked V2 cache segment for that hash type. Only hash
/// match types are ranked; a filename+size win surfaces the primary alone.
pub async fn identify_game_and_get_relations_v2(
	search: GameFileMatchSearch,
	redis_conn: &mut MultiplexedConnection,
	db_conn: &DbConn,
) -> anyhow::Result<CacheStatus<GameAndRelationMatchResultV2>> {
	let outcome = identify_game(&search, redis_conn, db_conn).await?;

	let (cached, hit) = match outcome {
		Cached(hit) => (true, hit),
		NonCached(hit) => (false, hit),
	};

	let Some((match_type, entry)) = hit else {
		let empty: GameAndRelationMatchResultV2 = empty_relation_match_result().into();
		return Ok(wrap_cache_status(cached, empty));
	};

	let primary_id = entry.game.id;
	let primary: GameAndRelationMatchResultV2 =
		build_relation_match_result(match_type, entry, db_conn)
			.await?
			.into();

	let additional_matches = match match_type.cache_segment() {
		Some(_) if match_type != GameMatchType::FileNameAndSize => {
			build_additional_matches_v2(match_type, &search, primary_id, redis_conn, db_conn)
				.await?
		}
		_ => Vec::new(),
	};

	let result = GameAndRelationMatchResultV2 {
		additional_matches,
		..primary
	};
	Ok(wrap_cache_status(cached, result))
}

fn wrap_cache_status<T>(cached: bool, value: T) -> CacheStatus<T> {
	if cached {
		Cached(value)
	} else {
		NonCached(value)
	}
}

/// Build the ranked siblings (every co-hashed game after element zero) for the
/// winning hash type. The primary is dropped by id rather than position so a
/// race that reorders the union cannot duplicate it.
async fn build_additional_matches_v2(
	match_type: GameMatchType,
	search: &GameFileMatchSearch,
	primary_id: Uuid,
	redis_conn: &mut MultiplexedConnection,
	db_conn: &DbConn,
) -> anyhow::Result<Vec<GameAndRelationsResultV2>> {
	// CRC is pinned to crc+size; the other hash rungs key on the hash alone.
	let ranked = match match_type {
		GameMatchType::CRC => match search.crc.as_deref() {
			Some(crc) => {
				find_all_games_and_metadata_ids_by_crc_size_cached(
					crc,
					search.file_size,
					redis_conn,
					db_conn,
				)
				.await?
			}
			None => return Ok(Vec::new()),
		},
		GameMatchType::SHA256 | GameMatchType::SHA1 | GameMatchType::MD5 => {
			let hash = match match_type {
				GameMatchType::SHA256 => search.sha256.as_deref(),
				GameMatchType::SHA1 => search.sha1.as_deref(),
				GameMatchType::MD5 => search.md5.as_deref(),
				_ => unreachable!("outer arm restricts to the three sha/md5 rungs"),
			};
			let Some(hash) = hash else {
				return Ok(Vec::new());
			};
			find_all_games_and_metadata_ids_by_hash_cached(hash, match_type, redis_conn, db_conn)
				.await?
		}
		GameMatchType::FileNameAndSize | GameMatchType::NoMatch => return Ok(Vec::new()),
	};
	let ranked = match ranked {
		Cached(v) | NonCached(v) => v,
	};

	let mut additional = Vec::new();
	for sibling in ranked.games {
		if sibling.game.id == primary_id {
			continue;
		}
		additional.push(build_relations_result_v2(sibling, db_conn).await?);
	}
	Ok(additional)
}

async fn build_relations_result_v2(
	entry: IdentifyEntry,
	db_conn: &DbConn,
) -> anyhow::Result<GameAndRelationsResultV2> {
	let game = get_game_by_id(entry.game.id, db_conn)
		.await?
		.unwrap_or(entry.game);

	let (dat_file_import, dat_file, signature_group, platform, company, game_files) =
		find_all_relations_of_game(&game, db_conn).await?;

	let latest = dat_file.latest_dat_file_import_id;
	let versions =
		versions_by_last_seen(game.last_seen_dat_file_import_id, &game_files, db_conn).await?;

	let result = GameAndRelationsResultBuilder::default()
		.game(enrich_game(game, latest, &versions))
		.platform(platform.into())
		.company(company.map(|c| c.into()))
		.game_files(
			game_files
				.into_iter()
				.map(|gf| enrich_game_file(gf, latest, &versions))
				.collect(),
		)
		.dat_file(dat_file.into())
		.dat_file_import(dat_file_import.into())
		.signature_group(signature_group.into())
		.external_metadata(
			entry
				.metadata_mappings
				.into_iter()
				.map(|m| m.into())
				.collect(),
		)
		.build()?;
	Ok(result.into())
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

/// Tries each match type in order (sha256, sha1, md5, crc, name+size) and
/// returns the first hit. When every attempted content key produced a
/// cached-but-empty result the outcome collapses to `Cached(None)`, otherwise
/// `NonCached(None)`.
async fn identify_game(
	search: &GameFileMatchSearch,
	redis_conn: &mut MultiplexedConnection,
	db_conn: &DbConn,
) -> anyhow::Result<CacheStatus<Option<(GameMatchType, IdentifyEntry)>>> {
	let started = std::time::Instant::now();
	let expected_count = [
		search.sha256.as_ref(),
		search.sha1.as_ref(),
		search.md5.as_ref(),
		search.crc.as_ref(),
	]
	.iter()
	.filter(|key| key.is_some())
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
			GameMatchType::CRC => match &search.crc {
				Some(crc) => {
					find_game_and_metadata_ids_by_crc_size_cached(
						crc,
						search.file_size,
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
			crate::metrics::record_identify_hit_position(match_type.metric_label());
			crate::metrics::observe_identify_latency("hit", started.elapsed().as_secs_f64());
			return Ok(result);
		}
	}

	let result = agg.finalize();
	if matches!(result, Cached(None)) {
		debug!("All possible game matches were cached as empty, returning cached no-match");
	}
	crate::metrics::observe_identify_latency("no_match", started.elapsed().as_secs_f64());
	Ok(result)
}

async fn build_relation_match_result(
	match_type: GameMatchType,
	entry: IdentifyEntry,
	db_conn: &DbConn,
) -> anyhow::Result<GameAndRelationMatchResult> {
	// Re-read the game so game-level lifecycle is accurate even when the entry
	// came from a cache hit written before a later import.
	let game = get_game_by_id(entry.game.id, db_conn)
		.await?
		.unwrap_or(entry.game);

	let (dat_file_import, dat_file, signature_group, platform, company, game_files) =
		find_all_relations_of_game(&game, db_conn).await?;

	let latest = dat_file.latest_dat_file_import_id;
	let versions =
		versions_by_last_seen(game.last_seen_dat_file_import_id, &game_files, db_conn).await?;

	Ok(GameAndRelationMatchResultBuilder::default()
		.game_match_type(match_type)
		.game(Some(enrich_game(game, latest, &versions)))
		.platform(Some(platform.into()))
		.company(company.map(|c| c.into()))
		.game_files(
			game_files
				.into_iter()
				.map(|gf| enrich_game_file(gf, latest, &versions))
				.collect(),
		)
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

async fn versions_by_last_seen(
	game_last_seen: Option<Uuid>,
	game_files: &[game_file::Model],
	db_conn: &DbConn,
) -> Result<HashMap<Uuid, String>, sea_orm::DbErr> {
	let mut ids: Vec<Uuid> = Vec::new();
	if let Some(id) = game_last_seen {
		ids.push(id);
	}
	for gf in game_files {
		if let Some(id) = gf.last_seen_dat_file_import_id {
			ids.push(id);
		}
	}
	ids.sort();
	ids.dedup();
	if ids.is_empty() {
		return Ok(HashMap::new());
	}

	let rows = dat_file_import::Entity::find()
		.filter(dat_file_import::Column::Id.is_in(ids))
		.all(db_conn)
		.await?;
	Ok(rows.into_iter().map(|r| (r.id, r.version)).collect())
}

/// Derive currency from the dat file's latest pointer rather than the cached
/// `is_current` flag, so the answer is correct even if that flag is briefly stale.
fn enrich_game(
	model: game::Model,
	latest: Option<Uuid>,
	versions: &HashMap<Uuid, String>,
) -> PlaymatchGame {
	let last_seen = model.last_seen_dat_file_import_id;
	let mut dto: PlaymatchGame = model.into();
	dto.current_in_latest_dat = last_seen.is_some() && last_seen == latest;
	dto.last_seen_dat_version = last_seen.and_then(|id| versions.get(&id).cloned());
	dto
}

fn enrich_game_file(
	model: game_file::Model,
	latest: Option<Uuid>,
	versions: &HashMap<Uuid, String>,
) -> PlaymatchGameFile {
	let last_seen = model.last_seen_dat_file_import_id;
	let mut dto: PlaymatchGameFile = model.into();
	dto.current_in_latest_dat = last_seen.is_some() && last_seen == latest;
	dto.last_seen_dat_version = last_seen.and_then(|id| versions.get(&id).cloned());
	dto
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
