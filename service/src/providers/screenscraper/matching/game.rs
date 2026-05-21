use crate::db::game::{
	get_automatic_match_failed_games_with_limit, get_unmatched_games_with_clone_of_with_limit,
	get_unmatched_games_without_clone_of_with_limit,
};
use crate::db::game_file::get_game_files_from_game_id;
use crate::db::platform::{
	find_platform_of_game, find_platform_related_signature_metadata_mapping,
};
use crate::matching::name_parse::parse_name;
use crate::matching::scoring::{
	CandidateGate, CandidateScore, Selection, gate_and_score, pick_best, record_pick_best,
};
use crate::matching::util::{clean_name, normalize_title};
use crate::providers::MetadataProvider;
use crate::providers::screenscraper::ScreenScraperClient;
use crate::providers::screenscraper::model::{REGION_PRIORITY, SsGame};
use crate::providers::{
	Target, drive_match_pipeline, write_auto_match_failed, write_auto_match_success,
};
use entity::game::Model;
use entity::sea_orm_active_enums::{
	AutomaticMatchReasonEnum, FailedMatchReasonEnum, MatchTypeEnum, MetadataProviderEnum,
};
use futures_util::future::BoxFuture;
use log::{debug, warn};
use sea_orm::DbConn;
use std::sync::Arc;

pub async fn match_games_to_screenscraper(
	client: Arc<ScreenScraperClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	let chunk_size = client.chunk_size();
	drive_match_pipeline(
		"game",
		MetadataProviderEnum::Screenscraper,
		get_unmatched_games_without_clone_of_with_limit,
		match_game_to_screenscraper,
		client.clone(),
		db_conn,
		chunk_size,
	)
	.await?;
	debug!("Finished matching games without clone_of id to ScreenScraper");

	if client.is_quota_exhausted() {
		warn!("ScreenScraper quota exhausted between game-match passes, ending cycle early");
		return Ok(());
	}

	drive_match_pipeline(
		"game",
		MetadataProviderEnum::Screenscraper,
		get_unmatched_games_with_clone_of_with_limit,
		match_clone_of_game_to_screenscraper,
		client.clone(),
		db_conn,
		chunk_size,
	)
	.await?;
	debug!("Finished matching games with clone_of id to ScreenScraper");

	if client.is_quota_exhausted() {
		warn!("ScreenScraper quota exhausted between game-match passes, ending cycle early");
		return Ok(());
	}

	drive_match_pipeline(
		"game",
		MetadataProviderEnum::Screenscraper,
		get_automatic_match_failed_games_with_limit,
		match_game_to_screenscraper,
		client,
		db_conn,
		chunk_size,
	)
	.await?;
	debug!("Finished retrying previously failed ScreenScraper game matches");

	Ok(())
}

fn match_clone_of_game_to_screenscraper(
	game: Model,
	client: Arc<ScreenScraperClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		if client.is_quota_exhausted() {
			return Ok(());
		}
		crate::providers::drive_clone_propagation(
			game,
			client,
			db_conn,
			match_game_to_screenscraper,
		)
		.await
	})
}

#[derive(Clone)]
struct ScoredCand {
	candidate_id: i64,
	matched_name: String,
	score: CandidateScore,
}

fn match_game_to_screenscraper(
	game: Model,
	client: Arc<ScreenScraperClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		if client.is_quota_exhausted() {
			return Ok(());
		}

		let mut redis_conn = client.redis_conn().clone();
		let system_id = get_game_platform_screenscraper_id(&game, &db_conn).await?;

		if let Some(()) =
			try_match_by_hashes(&game, system_id, &client, &db_conn, &mut redis_conn).await?
		{
			return Ok(());
		}

		if client.is_quota_exhausted() {
			return Ok(());
		}

		let parsed_dat = parse_name(&game.name);
		let cleaned = parsed_dat.base.to_lowercase();
		let cleaned_normalized = normalize_title(&cleaned);
		let dat_ss_regions: Vec<&'static str> = parsed_dat
			.regions
			.iter()
			.flat_map(|r| r.ss_codes().iter().copied())
			.collect();

		let candidates = client.search_games(system_id, &cleaned).await?;

		let (direct, normalized) = collect_ss_rungs(
			&candidates,
			&parsed_dat,
			&cleaned,
			&cleaned_normalized,
			&dat_ss_regions,
		);

		if let Some(()) = run_rung(
			&db_conn,
			&mut redis_conn,
			&game,
			record_pick_best(
				"screenscraper",
				"direct",
				pick_best(direct.iter().map(|s| (s, s.score))),
			),
			AutomaticMatchReasonEnum::DirectName,
			"Direct Match",
		)
		.await?
		{
			return Ok(());
		}
		if let Some(()) = run_rung(
			&db_conn,
			&mut redis_conn,
			&game,
			record_pick_best(
				"screenscraper",
				"normalized",
				pick_best(normalized.iter().map(|s| (s, s.score))),
			),
			AutomaticMatchReasonEnum::NormalizedName,
			"Normalized Match",
		)
		.await?
		{
			return Ok(());
		}

		debug!("No ScreenScraper match found for Game \"{}\"", &cleaned);
		write_auto_match_failed(
			"screenscraper",
			MetadataProviderEnum::Screenscraper,
			Target::Game(game.id),
			FailedMatchReasonEnum::NoDirectMatch,
			&db_conn,
			&mut redis_conn,
		)
		.await?;

		Ok(())
	})
}

fn collect_ss_rungs(
	candidates: &[SsGame],
	parsed_dat: &crate::matching::name_parse::ParsedName,
	cleaned: &str,
	cleaned_normalized: &str,
	dat_ss_regions: &[&str],
) -> (Vec<ScoredCand>, Vec<ScoredCand>) {
	let mut direct: Vec<ScoredCand> = vec![];
	let mut normalized: Vec<ScoredCand> = vec![];

	for c in candidates {
		let Some(candidate_id) = c.id else { continue };
		for nom in &c.noms {
			let parsed_cand = parse_name(&nom.text);
			let region_codes: [&str; 1] = [nom.region.as_str()];
			let gate = gate_and_score(
				parsed_dat,
				Some(&parsed_cand),
				None,
				&region_codes,
				REGION_PRIORITY,
				None,
				None,
				dat_ss_regions,
			);
			let score = match gate {
				CandidateGate::Reject => continue,
				CandidateGate::Pass(s) => s,
			};

			let lower = nom.text.to_lowercase();
			if lower == cleaned {
				direct.push(ScoredCand {
					candidate_id,
					matched_name: nom.text.clone(),
					score,
				});
				continue;
			}
			if normalize_title(&lower) == cleaned_normalized {
				normalized.push(ScoredCand {
					candidate_id,
					matched_name: nom.text.clone(),
					score,
				});
			}
		}
	}

	(direct, normalized)
}

async fn run_rung(
	db_conn: &DbConn,
	redis_conn: &mut redis::aio::MultiplexedConnection,
	game: &Model,
	selection: Selection<'_, ScoredCand>,
	reason: AutomaticMatchReasonEnum,
	label: &str,
) -> anyhow::Result<Option<()>> {
	match selection {
		Selection::Best(s) => {
			debug!(
				"Matched Game \"{}\" to ScreenScraper Game ID {} ({label})",
				&game.name, s.candidate_id
			);
			write_auto_match_success(
				"screenscraper",
				MetadataProviderEnum::Screenscraper,
				Target::Game(game.id),
				s.candidate_id.to_string(),
				reason,
				Some(s.matched_name.clone()),
				None,
				db_conn,
				redis_conn,
			)
			.await?;
			Ok(Some(()))
		}
		Selection::Ambiguous => {
			debug!(
				"Refusing to match Game \"{}\" on ScreenScraper: tied at top of score",
				&game.name
			);
			write_auto_match_failed(
				"screenscraper",
				MetadataProviderEnum::Screenscraper,
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

async fn try_match_by_hashes(
	game: &Model,
	system_id: i32,
	client: &ScreenScraperClient,
	db_conn: &DbConn,
	redis_conn: &mut redis::aio::MultiplexedConnection,
) -> anyhow::Result<Option<()>> {
	let files = get_game_files_from_game_id(game.id, db_conn).await?;
	if files.is_empty() {
		return Ok(None);
	}

	let parsed_dat = parse_name(&game.name);
	let dat_ss_regions: Vec<&'static str> = parsed_dat
		.regions
		.iter()
		.flat_map(|r| r.ss_codes().iter().copied())
		.collect();

	for file in &files {
		if client.is_quota_exhausted() {
			return Ok(None);
		}
		let rom_name = file.file_name.as_str();
		let rom_size = file.file_size_in_bytes;
		let sha1 = file
			.sha1
			.as_deref()
			.map(str::trim)
			.filter(|s| !s.is_empty());
		let md5 = file.md5.as_deref().map(str::trim).filter(|s| !s.is_empty());
		let crc = file.crc.as_deref().map(str::trim).filter(|s| !s.is_empty());

		let submitted = submitted_hashes(sha1, md5, crc);
		if submitted.is_empty() {
			continue;
		}

		let found = client
			.get_game_by_hashes(system_id, rom_name, rom_size, md5, sha1, crc)
			.await?;

		match found {
			Some(found) => {
				let winning = pick_winning_hash(&found, sha1, md5, crc, &submitted);
				record_hash_match(
					game,
					&found,
					reason_for(winning),
					&dat_ss_regions,
					db_conn,
					redis_conn,
				)
				.await?;
				for h in &submitted {
					let outcome = if *h == winning { "hit" } else { "miss" };
					crate::metrics::record_match_rung("screenscraper", rung_for(*h), outcome);
				}
				return Ok(Some(()));
			}
			None => {
				for h in &submitted {
					crate::metrics::record_match_rung("screenscraper", rung_for(*h), "miss");
				}
			}
		}
	}

	Ok(None)
}

/// `Ord` derive is load-bearing: variants are listed weakest to strongest so
/// `pick_winning_hash` can resolve which hash matched via `.max()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum HashKind {
	Crc,
	Md5,
	Sha1,
}

fn submitted_hashes(sha1: Option<&str>, md5: Option<&str>, crc: Option<&str>) -> Vec<HashKind> {
	let mut out = Vec::with_capacity(3);
	if sha1.is_some() {
		out.push(HashKind::Sha1);
	}
	if md5.is_some() {
		out.push(HashKind::Md5);
	}
	if crc.is_some() {
		out.push(HashKind::Crc);
	}
	out
}

fn rung_for(kind: HashKind) -> &'static str {
	match kind {
		HashKind::Sha1 => "sha1_hash",
		HashKind::Md5 => "md5_hash",
		HashKind::Crc => "crc_hash",
	}
}

fn reason_for(kind: HashKind) -> AutomaticMatchReasonEnum {
	match kind {
		HashKind::Sha1 => AutomaticMatchReasonEnum::Sha1Hash,
		HashKind::Md5 => AutomaticMatchReasonEnum::Md5Hash,
		HashKind::Crc => AutomaticMatchReasonEnum::CrcHash,
	}
}

/// ScreenScraper's response doesn't tag which hash matched, so we derive it
/// from the rom carrying a matching field. Falls back to the strongest hash
/// we submitted when `roms` is absent or no rom matches.
fn pick_winning_hash(
	found: &SsGame,
	sha1: Option<&str>,
	md5: Option<&str>,
	crc: Option<&str>,
	submitted: &[HashKind],
) -> HashKind {
	let strongest_submitted = *submitted
		.iter()
		.max()
		.expect("submitted must be non-empty when pick_winning_hash is called");

	let Some(roms) = found.roms.as_ref() else {
		return strongest_submitted;
	};

	let mut best: Option<HashKind> = None;
	let consider = |best: &mut Option<HashKind>, kind: HashKind| {
		if best.map(|b| kind > b).unwrap_or(true) {
			*best = Some(kind);
		}
	};

	for rom in roms {
		if let (Some(want), Some(got)) = (sha1, rom.romsha1.as_deref())
			&& want.eq_ignore_ascii_case(got)
		{
			consider(&mut best, HashKind::Sha1);
		}
		if let (Some(want), Some(got)) = (md5, rom.rommd5.as_deref())
			&& want.eq_ignore_ascii_case(got)
		{
			consider(&mut best, HashKind::Md5);
		}
		if let (Some(want), Some(got)) = (crc, rom.romcrc.as_deref())
			&& want.eq_ignore_ascii_case(got)
		{
			consider(&mut best, HashKind::Crc);
		}
	}

	best.unwrap_or(strongest_submitted)
}

async fn record_hash_match(
	game: &Model,
	found: &SsGame,
	reason: AutomaticMatchReasonEnum,
	dat_ss_regions: &[&str],
	db_conn: &DbConn,
	redis_conn: &mut redis::aio::MultiplexedConnection,
) -> anyhow::Result<()> {
	let Some(found_id) = found.id else {
		debug!(
			"ScreenScraper hash response missing id for Game \"{}\"; skipping",
			game.name
		);
		return Ok(());
	};
	debug!(
		"Matched Game \"{}\" to ScreenScraper Game ID {} ({:?})",
		game.name, found_id, reason
	);
	let matched_name = found
		.iter_candidate_names_with_region_priority(dat_ss_regions)
		.next()
		.map(str::to_string);
	write_auto_match_success(
		"screenscraper",
		MetadataProviderEnum::Screenscraper,
		Target::Game(game.id),
		found_id.to_string(),
		reason,
		matched_name,
		None,
		db_conn,
		redis_conn,
	)
	.await
}

async fn get_game_platform_screenscraper_id(game: &Model, db_conn: &DbConn) -> anyhow::Result<i32> {
	let platform = find_platform_of_game(game.id, db_conn)
		.await?
		.ok_or_else(|| {
			anyhow::anyhow!(
				"No platform found for Game \"{}\", this shouldn't happen...",
				game.name
			)
		})?;

	let mapping = find_platform_related_signature_metadata_mapping(
		&platform,
		MetadataProviderEnum::Screenscraper,
		db_conn,
	)
	.await?
	.ok_or_else(|| {
		anyhow::anyhow!(
			"Platform {} is missing its screenscraper metadata mapping, this shouldn't happen...",
			&platform.name
		)
	})?;

	if !matches!(
		mapping.match_type,
		MatchTypeEnum::Automatic | MatchTypeEnum::Manual
	) {
		return Err(anyhow::anyhow!(
			"Platform {} is not matched to ScreenScraper, this shouldn't happen...",
			&platform.name
		));
	}

	let provider_id = mapping.provider_id.ok_or_else(|| {
		anyhow::anyhow!(
			"Platform {} screenscraper mapping has no provider_id",
			&platform.name
		)
	})?;
	provider_id.parse::<i32>().map_err(|e| {
		anyhow::anyhow!(
			"Platform {} screenscraper provider_id {provider_id:?} is not a valid i32: {e}",
			&platform.name
		)
	})
}

pub fn match_game_via_sibling_name_screenscraper(
	game: Model,
	sibling_names: Vec<String>,
	client: Arc<ScreenScraperClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		if client.is_quota_exhausted() {
			return Ok(());
		}
		let mut redis_conn = client.redis_conn().clone();
		let system_id = get_game_platform_screenscraper_id(&game, &db_conn).await?;
		let parsed_dat = parse_name(&game.name);
		let cleaned_playmatch = parsed_dat.base.to_lowercase();
		let dat_ss_regions: Vec<&'static str> = parsed_dat
			.regions
			.iter()
			.flat_map(|r| r.ss_codes().iter().copied())
			.collect();
		let mut tried: std::collections::HashSet<String> = std::collections::HashSet::new();
		tried.insert(cleaned_playmatch);

		for sibling in sibling_names {
			if client.is_quota_exhausted() {
				return Ok(());
			}
			let q = clean_name(&sibling).to_lowercase();
			if !tried.insert(q.clone()) {
				continue;
			}
			let q_norm = normalize_title(&q);
			let candidates = client.search_games(system_id, &q).await?;

			let (direct, normalized) =
				collect_ss_rungs(&candidates, &parsed_dat, &q, &q_norm, &dat_ss_regions);

			if try_write_cross(
				&db_conn,
				&mut redis_conn,
				&game,
				&sibling,
				record_pick_best(
					"screenscraper",
					"cross_direct",
					pick_best(direct.iter().map(|s| (s, s.score))),
				),
				AutomaticMatchReasonEnum::CrossProviderDirectName,
				"Direct",
			)
			.await?
			{
				return Ok(());
			}
			if try_write_cross(
				&db_conn,
				&mut redis_conn,
				&game,
				&sibling,
				record_pick_best(
					"screenscraper",
					"cross_normalized",
					pick_best(normalized.iter().map(|s| (s, s.score))),
				),
				AutomaticMatchReasonEnum::CrossProviderNormalizedName,
				"Normalized",
			)
			.await?
			{
				return Ok(());
			}
		}
		Ok(())
	})
}

async fn try_write_cross(
	db_conn: &DbConn,
	redis_conn: &mut redis::aio::MultiplexedConnection,
	game: &Model,
	sibling: &str,
	selection: Selection<'_, ScoredCand>,
	reason: AutomaticMatchReasonEnum,
	label: &str,
) -> anyhow::Result<bool> {
	match selection {
		Selection::Best(s) => {
			debug!(
				"Cross-matched Game \"{}\" to ScreenScraper Game ID {} via sibling \"{}\" ({label})",
				&game.name, s.candidate_id, sibling
			);
			write_auto_match_success(
				"screenscraper",
				MetadataProviderEnum::Screenscraper,
				Target::Game(game.id),
				s.candidate_id.to_string(),
				reason,
				Some(s.matched_name.clone()),
				None,
				db_conn,
				redis_conn,
			)
			.await?;
			Ok(true)
		}
		Selection::Ambiguous => {
			write_auto_match_failed(
				"screenscraper",
				MetadataProviderEnum::Screenscraper,
				Target::Game(game.id),
				FailedMatchReasonEnum::Ambiguous,
				db_conn,
				redis_conn,
			)
			.await?;
			Ok(true)
		}
		Selection::None => Ok(false),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::providers::screenscraper::model::SsRom;

	fn ss_game(roms: Option<Vec<SsRom>>) -> SsGame {
		SsGame {
			id: Some(1),
			noms: vec![],
			roms,
			editeur: None,
			developpeur: None,
		}
	}

	fn rom(sha1: Option<&str>, md5: Option<&str>, crc: Option<&str>) -> SsRom {
		SsRom {
			romfilename: None,
			rommd5: md5.map(str::to_string),
			romsha1: sha1.map(str::to_string),
			romcrc: crc.map(str::to_string),
		}
	}

	#[test]
	fn pick_winning_hash_prefers_sha1_when_response_carries_it() {
		let sha1 = "AABBCC";
		let md5 = "112233";
		let crc = "DEADBEEF";
		let game = ss_game(Some(vec![rom(Some(sha1), Some(md5), Some(crc))]));
		let submitted = submitted_hashes(Some(sha1), Some(md5), Some(crc));
		assert_eq!(
			pick_winning_hash(&game, Some(sha1), Some(md5), Some(crc), &submitted),
			HashKind::Sha1,
		);
	}

	#[test]
	fn pick_winning_hash_picks_md5_when_only_md5_matches_in_response() {
		let md5 = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
		let game = ss_game(Some(vec![rom(None, Some(md5), None)]));
		let submitted = submitted_hashes(Some("noop_sha1"), Some(md5), Some("noop_crc"));
		assert_eq!(
			pick_winning_hash(
				&game,
				Some("noop_sha1"),
				Some(md5),
				Some("noop_crc"),
				&submitted
			),
			HashKind::Md5,
		);
	}

	#[test]
	fn pick_winning_hash_is_case_insensitive() {
		let sha1_upper = "ABCDEF1234567890ABCDEF1234567890ABCDEF12";
		let sha1_lower = sha1_upper.to_lowercase();
		let game = ss_game(Some(vec![rom(Some(&sha1_lower), None, None)]));
		let submitted = submitted_hashes(Some(sha1_upper), None, None);
		assert_eq!(
			pick_winning_hash(&game, Some(sha1_upper), None, None, &submitted),
			HashKind::Sha1,
		);
	}

	#[test]
	fn pick_winning_hash_falls_back_to_strongest_submitted_when_response_has_no_roms() {
		let game = ss_game(None);
		let submitted = submitted_hashes(None, Some("md5"), Some("crc"));
		assert_eq!(
			pick_winning_hash(&game, None, Some("md5"), Some("crc"), &submitted),
			HashKind::Md5,
		);
	}

	#[test]
	fn pick_winning_hash_falls_back_when_no_rom_matches() {
		let game = ss_game(Some(vec![rom(Some("zzz"), Some("yyy"), Some("xxx"))]));
		let submitted = submitted_hashes(Some("aaa"), None, Some("bbb"));
		assert_eq!(
			pick_winning_hash(&game, Some("aaa"), None, Some("bbb"), &submitted),
			HashKind::Sha1,
		);
	}

	#[test]
	fn submitted_hashes_orders_strongest_first() {
		let submitted = submitted_hashes(Some("s"), Some("m"), Some("c"));
		assert_eq!(
			submitted,
			vec![HashKind::Sha1, HashKind::Md5, HashKind::Crc]
		);
	}

	#[test]
	fn submitted_hashes_skips_absent_inputs() {
		let submitted = submitted_hashes(None, Some("m"), None);
		assert_eq!(submitted, vec![HashKind::Md5]);
	}
}
