use crate::db::game::{
	get_automatic_match_failed_games_with_limit,
	get_unmatched_games_with_clone_of_with_limit_no_platform_gate,
	get_unmatched_games_without_clone_of_with_limit_no_platform_gate,
};
use crate::db::game_file::get_game_files_from_game_id;
use crate::db::openvgdb::{
	find_openvgdb_releases_by_title_lower, find_openvgdb_releases_by_title_normalized,
	find_openvgdb_releases_for_rom, find_openvgdb_rom_by_crc, find_openvgdb_rom_by_md5,
	find_openvgdb_rom_by_sha1,
};
use crate::matching::name_parse::{ParsedName, parse_name};
use crate::matching::scoring::{
	CandidateGate, CandidateScore, Selection, gate_and_score, pick_best, record_pick_best,
};
use crate::matching::util::normalize_title;
use crate::providers::openvgdb::OpenVgdbClient;
use crate::providers::{
	DEFAULT_CHUNK_SIZE, MetadataProvider, Target, drive_match_pipeline, write_auto_match_failed,
	write_auto_match_success,
};
use entity::game::Model;
use entity::sea_orm_active_enums::{
	AutomaticMatchReasonEnum, FailedMatchReasonEnum, MetadataProviderEnum,
};
use entity::{openvgdb_release, openvgdb_rom};
use futures_util::future::BoxFuture;
use log::debug;
use redis::aio::MultiplexedConnection;
use sea_orm::DbConn;
use std::sync::Arc;

pub async fn match_games_to_openvgdb(
	client: Arc<OpenVgdbClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	drive_match_pipeline(
		"game",
		MetadataProviderEnum::OpenVGDB,
		get_unmatched_games_without_clone_of_with_limit_no_platform_gate,
		match_game_to_openvgdb,
		client.clone(),
		db_conn,
		DEFAULT_CHUNK_SIZE,
	)
	.await?;
	debug!("Finished matching games without clone_of id to OpenVGDB");

	drive_match_pipeline(
		"game",
		MetadataProviderEnum::OpenVGDB,
		get_unmatched_games_with_clone_of_with_limit_no_platform_gate,
		match_clone_of_game_to_openvgdb,
		client.clone(),
		db_conn,
		DEFAULT_CHUNK_SIZE,
	)
	.await?;
	debug!("Finished matching games with clone_of id to OpenVGDB");

	drive_match_pipeline(
		"game",
		MetadataProviderEnum::OpenVGDB,
		get_automatic_match_failed_games_with_limit,
		match_game_to_openvgdb,
		client,
		db_conn,
		DEFAULT_CHUNK_SIZE,
	)
	.await?;
	debug!("Finished retrying previously failed OpenVGDB game matches");

	Ok(())
}

fn match_clone_of_game_to_openvgdb(
	game: Model,
	client: Arc<OpenVgdbClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		crate::providers::drive_clone_propagation(game, client, db_conn, match_game_to_openvgdb)
			.await
	})
}

fn match_game_to_openvgdb(
	game: Model,
	client: Arc<OpenVgdbClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		let mut redis_conn = client.redis_conn().clone();

		let parsed_dat = parse_name(&game.name);
		let dat_regions: Vec<&'static str> = parsed_dat
			.regions
			.iter()
			.flat_map(|r| r.ovgdb_codes().iter().copied())
			.collect();

		if let Some(()) =
			try_match_by_hashes(&game, &dat_regions, &db_conn, &mut redis_conn).await?
		{
			return Ok(());
		}

		// Name fallback: when hash matching misses (homebrew rebuild,
		// header-stripped variant, dump newer than the OpenVGDB snapshot),
		// fall through to title-name lookups against the imported releases.
		let cleaned = parsed_dat.base.to_lowercase();
		let cleaned_normalized = normalize_title(&cleaned);

		let releases =
			find_openvgdb_releases_by_title_lower(&cleaned, &dat_regions, &db_conn).await?;
		let scored = score_ovgdb_releases(&parsed_dat, &releases);
		if let Some(()) = run_ovgdb_name_rung(
			&db_conn,
			&mut redis_conn,
			&game,
			record_pick_best(
				"openvgdb",
				"direct",
				pick_best(scored.iter().map(|s| (s, s.score))),
			),
			AutomaticMatchReasonEnum::DirectName,
			"Direct Name",
		)
		.await?
		{
			return Ok(());
		}

		let releases =
			find_openvgdb_releases_by_title_normalized(&cleaned_normalized, &dat_regions, &db_conn)
				.await?;
		let scored = score_ovgdb_releases(&parsed_dat, &releases);
		if let Some(()) = run_ovgdb_name_rung(
			&db_conn,
			&mut redis_conn,
			&game,
			record_pick_best(
				"openvgdb",
				"normalized",
				pick_best(scored.iter().map(|s| (s, s.score))),
			),
			AutomaticMatchReasonEnum::NormalizedName,
			"Normalized Name",
		)
		.await?
		{
			return Ok(());
		}

		debug!("No OpenVGDB match found for Game \"{}\"", &game.name);
		write_auto_match_failed(
			"openvgdb",
			MetadataProviderEnum::OpenVGDB,
			Target::Game(game.id),
			FailedMatchReasonEnum::NoDirectMatch,
			&db_conn,
			&mut redis_conn,
		)
		.await?;

		Ok(())
	})
}

#[derive(Clone)]
struct ScoredRelease<'a> {
	release: &'a openvgdb_release::Model,
	score: CandidateScore,
}

fn score_ovgdb_releases<'a>(
	parsed_dat: &ParsedName,
	releases: &'a [openvgdb_release::Model],
) -> Vec<ScoredRelease<'a>> {
	releases
		.iter()
		.filter_map(|r| {
			let parsed_cand = parse_name(&r.title_name);
			let cand_year = r.release_year.and_then(|y| u16::try_from(y).ok());
			match gate_and_score(
				parsed_dat,
				Some(&parsed_cand),
				cand_year,
				&[],
				&[],
				None,
				None,
				&[],
			) {
				CandidateGate::Reject => None,
				CandidateGate::Pass(score) => Some(ScoredRelease { release: r, score }),
			}
		})
		.collect()
}

async fn run_ovgdb_name_rung(
	db_conn: &DbConn,
	redis_conn: &mut MultiplexedConnection,
	game: &Model,
	selection: Selection<'_, ScoredRelease<'_>>,
	reason: AutomaticMatchReasonEnum,
	label: &str,
) -> anyhow::Result<Option<()>> {
	match selection {
		Selection::Best(s) => {
			debug!(
				"Matched Game \"{}\" to OpenVGDB Release ID {} ({label})",
				game.name, s.release.release_id
			);
			write_auto_match_success(
				"openvgdb",
				MetadataProviderEnum::OpenVGDB,
				Target::Game(game.id),
				s.release.release_id.to_string(),
				reason,
				Some(s.release.title_name.clone()),
				s.release.release_year.and_then(|y| i16::try_from(y).ok()),
				db_conn,
				redis_conn,
			)
			.await?;
			Ok(Some(()))
		}
		Selection::Ambiguous => {
			debug!(
				"Refusing to match Game \"{}\" on OpenVGDB: tied at top of score",
				game.name
			);
			write_auto_match_failed(
				"openvgdb",
				MetadataProviderEnum::OpenVGDB,
				Target::Game(game.id),
				FailedMatchReasonEnum::Ambiguous,
				db_conn,
				redis_conn,
			)
			.await?;
			Ok(Some(()))
		}
		Selection::None => Ok(None),
	}
}

/// Walks every game file's sha1/md5/crc against the imported `openvgdb_rom`
/// table, returning `Some(())` once a hash hits a rom that carries at least
/// one release.
async fn try_match_by_hashes(
	game: &Model,
	dat_regions: &[&str],
	db_conn: &DbConn,
	redis_conn: &mut redis::aio::MultiplexedConnection,
) -> anyhow::Result<Option<()>> {
	let files = get_game_files_from_game_id(game.id, db_conn).await?;

	for file in &files {
		if let Some(sha1) = file
			.sha1
			.as_deref()
			.map(str::trim)
			.filter(|s| !s.is_empty())
		{
			let hit = match find_openvgdb_rom_by_sha1(sha1, db_conn).await? {
				Some(rom) => record_hash_match(
					game,
					&rom,
					AutomaticMatchReasonEnum::Sha1Hash,
					dat_regions,
					db_conn,
					redis_conn,
				)
				.await?
				.is_some(),
				None => false,
			};
			crate::metrics::record_match_rung(
				"openvgdb",
				"sha1_hash",
				if hit { "hit" } else { "miss" },
			);
			if hit {
				return Ok(Some(()));
			}
		}
		if let Some(md5) = file.md5.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
			let hit = match find_openvgdb_rom_by_md5(md5, db_conn).await? {
				Some(rom) => record_hash_match(
					game,
					&rom,
					AutomaticMatchReasonEnum::Md5Hash,
					dat_regions,
					db_conn,
					redis_conn,
				)
				.await?
				.is_some(),
				None => false,
			};
			crate::metrics::record_match_rung(
				"openvgdb",
				"md5_hash",
				if hit { "hit" } else { "miss" },
			);
			if hit {
				return Ok(Some(()));
			}
		}
		if let Some(crc) = file.crc.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
			let hit = match find_openvgdb_rom_by_crc(crc, db_conn).await? {
				Some(rom) => record_hash_match(
					game,
					&rom,
					AutomaticMatchReasonEnum::CrcHash,
					dat_regions,
					db_conn,
					redis_conn,
				)
				.await?
				.is_some(),
				None => false,
			};
			crate::metrics::record_match_rung(
				"openvgdb",
				"crc_hash",
				if hit { "hit" } else { "miss" },
			);
			if hit {
				return Ok(Some(()));
			}
		}
	}

	Ok(None)
}

/// Picks the best release for a matched rom (region priority) and records the
/// mapping. Returns `None` when the rom carries no releases so the caller can
/// keep trying other hashes.
async fn record_hash_match(
	game: &Model,
	rom: &openvgdb_rom::Model,
	reason: AutomaticMatchReasonEnum,
	dat_regions: &[&str],
	db_conn: &DbConn,
	redis_conn: &mut redis::aio::MultiplexedConnection,
) -> anyhow::Result<Option<()>> {
	let releases = find_openvgdb_releases_for_rom(rom.rom_id, dat_regions, db_conn).await?;
	let Some(release) = releases.into_iter().next() else {
		return Ok(None);
	};

	debug!(
		"Matched Game \"{}\" to OpenVGDB Release ID {} ({:?})",
		game.name, release.release_id, reason
	);
	write_auto_match_success(
		"openvgdb",
		MetadataProviderEnum::OpenVGDB,
		Target::Game(game.id),
		release.release_id.to_string(),
		reason,
		Some(release.title_name),
		release.release_year.and_then(|y| i16::try_from(y).ok()),
		db_conn,
		redis_conn,
	)
	.await?;
	Ok(Some(()))
}

#[cfg(test)]
mod tests {
	use super::*;
	use chrono::Utc;
	use sea_orm::prelude::Uuid;

	fn ovgdb_release(release_id: i64, title: &str, year: Option<i32>) -> openvgdb_release::Model {
		openvgdb_release::Model {
			id: Uuid::nil(),
			release_id,
			rom_id: 0,
			title_name: title.to_string(),
			title_name_normalized: Some(normalize_title(&title.to_lowercase())),
			region_name: None,
			system_name: None,
			cover_front: None,
			cover_back: None,
			description: None,
			developer: None,
			publisher: None,
			genre: None,
			release_date: None,
			release_year: year,
			reference_url: None,
			created_at: Utc::now().fixed_offset(),
		}
	}

	#[test]
	fn score_ovgdb_releases_picks_year_match() {
		let parsed_dat = parse_name("Pokemon Diamond Version (USA) (2006)");
		let releases = vec![
			ovgdb_release(1, "Pokemon Diamond Version", None),
			ovgdb_release(2, "Pokemon Diamond Version", Some(2006)),
		];
		let scored = score_ovgdb_releases(&parsed_dat, &releases);
		let selection = pick_best(scored.iter().map(|s| (s, s.score)));
		match selection {
			Selection::Best(s) => assert_eq!(s.release.release_id, 2),
			Selection::Ambiguous => panic!("expected Best, got Ambiguous"),
			Selection::None => panic!("expected Best, got None"),
		}
	}

	#[test]
	fn score_ovgdb_releases_returns_ambiguous_on_tied_top() {
		let parsed_dat = parse_name("Pokemon Diamond Version (USA)");
		let releases = vec![
			ovgdb_release(10, "Pokemon Diamond Version", None),
			ovgdb_release(20, "Pokemon Diamond Version", None),
		];
		let scored = score_ovgdb_releases(&parsed_dat, &releases);
		let selection = pick_best(scored.iter().map(|s| (s, s.score)));
		assert!(matches!(selection, Selection::Ambiguous));
	}
}
