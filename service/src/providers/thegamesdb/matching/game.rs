use crate::db::game::{
	get_automatic_match_failed_games_with_limit,
	get_unmatched_games_with_clone_of_with_limit_no_platform_gate,
	get_unmatched_games_without_clone_of_with_limit_no_platform_gate,
};
use crate::db::thegamesdb::{
	find_tgdb_games_by_alias_name, find_tgdb_games_by_alias_name_normalized,
	find_tgdb_games_by_title, find_tgdb_games_by_title_normalized, insert_tgdb_aliases,
	upsert_tgdb_game,
};
use crate::matching::name_parse::{ParsedName, parse_name};
use crate::matching::scoring::{
	CandidateGate, CandidateScore, Selection, gate_and_score, pick_best, record_pick_best,
};
use crate::matching::util::{clean_name, normalize_title};
use crate::providers::MetadataProvider;
use crate::providers::thegamesdb::TheGamesDbClient;
use crate::providers::thegamesdb::api::{SearchOutcome, SearchResult};
use crate::providers::thegamesdb::model::TgdbApiGame;
use crate::providers::{
	DEFAULT_CHUNK_SIZE, Target, drive_match_pipeline, write_auto_match_failed,
	write_auto_match_success,
};
use entity::game::Model;
use entity::sea_orm_active_enums::{
	AutomaticMatchReasonEnum, FailedMatchReasonEnum, MetadataProviderEnum,
};
use entity::tgdb_game;
use futures_util::future::BoxFuture;
use log::debug;
use redis::aio::MultiplexedConnection;
use sea_orm::DbConn;
use std::sync::Arc;

pub async fn match_games_to_thegamesdb(
	client: Arc<TheGamesDbClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	drive_match_pipeline(
		"game",
		MetadataProviderEnum::TheGamesDB,
		get_unmatched_games_without_clone_of_with_limit_no_platform_gate,
		match_game_to_thegamesdb,
		client.clone(),
		db_conn,
		DEFAULT_CHUNK_SIZE,
	)
	.await?;
	debug!("Finished matching games without clone_of id to TheGamesDB");

	drive_match_pipeline(
		"game",
		MetadataProviderEnum::TheGamesDB,
		get_unmatched_games_with_clone_of_with_limit_no_platform_gate,
		match_clone_of_game_to_thegamesdb,
		client.clone(),
		db_conn,
		DEFAULT_CHUNK_SIZE,
	)
	.await?;
	debug!("Finished matching games with clone_of id to TheGamesDB");

	drive_match_pipeline(
		"game",
		MetadataProviderEnum::TheGamesDB,
		get_automatic_match_failed_games_with_limit,
		match_game_to_thegamesdb,
		client,
		db_conn,
		DEFAULT_CHUNK_SIZE,
	)
	.await?;
	debug!("Finished retrying previously failed TheGamesDB game matches");

	Ok(())
}

// TGDB stores a separate row per regional release (for example the iQue
// Chinese Super Mario 64 DS is id 105388, while the world release is 110420).
// Cross-clone propagation would let a region-specific child match stamp the
// parent (or a world-release parent stamp every regional child), so clones
// match independently here instead of going through drive_clone_propagation.
fn match_clone_of_game_to_thegamesdb(
	game: Model,
	client: Arc<TheGamesDbClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	match_game_to_thegamesdb(game, client, db_conn)
}

#[derive(Clone)]
struct ScoredCand<'a> {
	cand: &'a tgdb_game::Model,
	score: CandidateScore,
}

fn score_local_candidates<'a>(
	parsed_dat: &ParsedName,
	candidates: &'a [tgdb_game::Model],
) -> Vec<ScoredCand<'a>> {
	candidates
		.iter()
		.filter_map(|c| {
			let parsed_cand = parse_name(&c.title);
			match gate_and_score(
				parsed_dat,
				Some(&parsed_cand),
				None,
				&[],
				&[],
				None,
				None,
				&[],
			) {
				CandidateGate::Reject => None,
				CandidateGate::Pass(score) => Some(ScoredCand { cand: c, score }),
			}
		})
		.collect()
}

struct ScoredApiCand<'a> {
	cand: &'a TgdbApiGame,
	score: CandidateScore,
}

fn score_api_candidates<'a>(
	parsed_dat: &ParsedName,
	cleaned: &str,
	cleaned_normalized: &str,
	candidates: &'a [TgdbApiGame],
) -> Vec<ScoredApiCand<'a>> {
	candidates
		.iter()
		.filter_map(|c| {
			let title = c.game_title.as_deref()?;
			let mut names = vec![title.to_string()];
			if let Some(alts) = &c.alternates {
				names.extend(alts.iter().cloned());
			}
			let matches_any = names.iter().any(|n| {
				let lower = n.to_lowercase();
				lower == cleaned || normalize_title(&lower) == cleaned_normalized
			});
			if !matches_any {
				return None;
			}
			let parsed_cand = parse_name(title);
			match gate_and_score(
				parsed_dat,
				Some(&parsed_cand),
				None,
				&[],
				&[],
				None,
				None,
				&[],
			) {
				CandidateGate::Reject => None,
				CandidateGate::Pass(score) => Some(ScoredApiCand { cand: c, score }),
			}
		})
		.collect()
}

fn match_game_to_thegamesdb(
	game: Model,
	client: Arc<TheGamesDbClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		let mut redis_conn = client.redis_conn().clone();
		let parsed_dat = parse_name(&game.name);
		let cleaned = parsed_dat.base.to_lowercase();
		let cleaned_normalized = normalize_title(&cleaned);

		let candidates = find_tgdb_games_by_title(&cleaned, &db_conn).await?;
		let scored = score_local_candidates(&parsed_dat, &candidates);
		if let Some(()) = run_local_rung(
			&db_conn,
			&mut redis_conn,
			&game,
			record_pick_best(
				"thegamesdb",
				"direct",
				pick_best(scored.iter().map(|s| (s, s.score))),
			),
			AutomaticMatchReasonEnum::DirectName,
			"Direct Match",
		)
		.await?
		{
			return Ok(());
		}

		let candidates = find_tgdb_games_by_alias_name(&cleaned, &db_conn).await?;
		let scored = score_local_candidates(&parsed_dat, &candidates);
		if let Some(()) = run_local_rung(
			&db_conn,
			&mut redis_conn,
			&game,
			record_pick_best(
				"thegamesdb",
				"alternative",
				pick_best(scored.iter().map(|s| (s, s.score))),
			),
			AutomaticMatchReasonEnum::AlternativeName,
			"Alternative Name",
		)
		.await?
		{
			return Ok(());
		}

		let candidates = find_tgdb_games_by_title_normalized(&cleaned_normalized, &db_conn).await?;
		let scored = score_local_candidates(&parsed_dat, &candidates);
		if let Some(()) = run_local_rung(
			&db_conn,
			&mut redis_conn,
			&game,
			record_pick_best(
				"thegamesdb",
				"normalized",
				pick_best(scored.iter().map(|s| (s, s.score))),
			),
			AutomaticMatchReasonEnum::NormalizedName,
			"Normalized Match",
		)
		.await?
		{
			return Ok(());
		}

		let candidates =
			find_tgdb_games_by_alias_name_normalized(&cleaned_normalized, &db_conn).await?;
		let scored = score_local_candidates(&parsed_dat, &candidates);
		if let Some(()) = run_local_rung(
			&db_conn,
			&mut redis_conn,
			&game,
			record_pick_best(
				"thegamesdb",
				"normalized_alternative",
				pick_best(scored.iter().map(|s| (s, s.score))),
			),
			AutomaticMatchReasonEnum::NormalizedAlternativeName,
			"Normalized Alternative Name",
		)
		.await?
		{
			return Ok(());
		}

		if client.has_api_key() {
			let SearchResult { outcome, games } = client.search_by_name(&cleaned).await?;
			match outcome {
				SearchOutcome::Hit => {
					let scored =
						score_api_candidates(&parsed_dat, &cleaned, &cleaned_normalized, &games);
					if let Some(()) = run_api_rung(
						&client,
						&db_conn,
						&mut redis_conn,
						&game,
						record_pick_best(
							"thegamesdb",
							"api_search",
							pick_best(scored.iter().map(|s| (s, s.score))),
						),
					)
					.await?
					{
						return Ok(());
					}
				}
				SearchOutcome::Miss | SearchOutcome::QuotaExhausted => {}
			}
		}

		debug!("No TheGamesDB match found for Game \"{}\"", &cleaned);
		write_auto_match_failed(
			"thegamesdb",
			MetadataProviderEnum::TheGamesDB,
			Target::Game(game.id),
			FailedMatchReasonEnum::NoDirectMatch,
			&db_conn,
			&mut redis_conn,
		)
		.await?;

		Ok(())
	})
}

async fn run_local_rung(
	db_conn: &DbConn,
	redis_conn: &mut MultiplexedConnection,
	game: &Model,
	selection: Selection<'_, ScoredCand<'_>>,
	reason: AutomaticMatchReasonEnum,
	label: &str,
) -> anyhow::Result<Option<()>> {
	match selection {
		Selection::Best(s) => {
			debug!(
				"Matched Game \"{}\" to TheGamesDB Game ID {} ({label})",
				&game.name, s.cand.id
			);
			write_auto_match_success(
				"thegamesdb",
				MetadataProviderEnum::TheGamesDB,
				Target::Game(game.id),
				s.cand.id.to_string(),
				reason,
				Some(s.cand.title.clone()),
				None,
				db_conn,
				redis_conn,
			)
			.await?;
			Ok(Some(()))
		}
		Selection::Ambiguous => {
			write_auto_match_failed(
				"thegamesdb",
				MetadataProviderEnum::TheGamesDB,
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

async fn run_api_rung(
	client: &TheGamesDbClient,
	db_conn: &DbConn,
	redis_conn: &mut MultiplexedConnection,
	game: &Model,
	selection: Selection<'_, ScoredApiCand<'_>>,
) -> anyhow::Result<Option<()>> {
	match selection {
		Selection::Best(s) => {
			let api_game = s.cand;
			let title = api_game.game_title.clone().unwrap_or_default();
			let title_lower = title.to_lowercase();
			let title_normalized = normalize_title(&title_lower);
			let normalized_opt = if title_normalized.trim().is_empty() {
				None
			} else {
				Some(title_normalized)
			};

			if let Err(e) = upsert_tgdb_game(
				api_game.id,
				&title,
				normalized_opt.as_deref(),
				client.db_conn(),
			)
			.await
			{
				debug!(
					"tgdb upsert from api hit failed for id {}: {e}",
					api_game.id
				);
			} else if let Some(alts) = &api_game.alternates {
				let alias_rows: Vec<(String, Option<String>)> = alts
					.iter()
					.filter(|a| !a.trim().is_empty())
					.map(|a| {
						let normalized = normalize_title(&a.to_lowercase());
						let norm = if normalized.trim().is_empty() {
							None
						} else {
							Some(normalized)
						};
						(a.clone(), norm)
					})
					.collect();
				if !alias_rows.is_empty()
					&& let Err(e) =
						insert_tgdb_aliases(api_game.id, &alias_rows, client.db_conn()).await
				{
					debug!(
						"tgdb alias insert from api hit failed for id {}: {e}",
						api_game.id
					);
				}
			}

			debug!(
				"Matched Game \"{}\" to TheGamesDB Game ID {} (API Search)",
				&game.name, api_game.id
			);
			write_auto_match_success(
				"thegamesdb",
				MetadataProviderEnum::TheGamesDB,
				Target::Game(game.id),
				api_game.id.to_string(),
				AutomaticMatchReasonEnum::DirectName,
				Some(title),
				None,
				db_conn,
				redis_conn,
			)
			.await?;
			Ok(Some(()))
		}
		Selection::Ambiguous => {
			write_auto_match_failed(
				"thegamesdb",
				MetadataProviderEnum::TheGamesDB,
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

pub fn match_game_via_sibling_name_thegamesdb(
	game: Model,
	sibling_names: Vec<String>,
	client: Arc<TheGamesDbClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		let mut redis_conn = client.redis_conn().clone();
		let parsed_dat = parse_name(&game.name);
		let cleaned_playmatch = parsed_dat.base.to_lowercase();
		let normalized_playmatch = normalize_title(&cleaned_playmatch);
		let mut tried: std::collections::HashSet<String> = std::collections::HashSet::new();
		tried.insert(cleaned_playmatch);
		tried.insert(normalized_playmatch);

		for sibling in sibling_names {
			let q = clean_name(&sibling).to_lowercase();
			if !tried.insert(q.clone()) {
				continue;
			}
			let q_norm = normalize_title(&q);
			tried.insert(q_norm.clone());

			let candidates = find_tgdb_games_by_title(&q, &db_conn).await?;
			let scored = score_local_candidates(&parsed_dat, &candidates);
			if try_write_cross(
				&db_conn,
				&mut redis_conn,
				&game,
				&sibling,
				record_pick_best(
					"thegamesdb",
					"cross_direct",
					pick_best(scored.iter().map(|s| (s, s.score))),
				),
				AutomaticMatchReasonEnum::CrossProviderDirectName,
				"Direct",
			)
			.await?
			{
				return Ok(());
			}

			let candidates = find_tgdb_games_by_alias_name(&q, &db_conn).await?;
			let scored = score_local_candidates(&parsed_dat, &candidates);
			if try_write_cross(
				&db_conn,
				&mut redis_conn,
				&game,
				&sibling,
				record_pick_best(
					"thegamesdb",
					"cross_alternative",
					pick_best(scored.iter().map(|s| (s, s.score))),
				),
				AutomaticMatchReasonEnum::CrossProviderDirectName,
				"Direct alt",
			)
			.await?
			{
				return Ok(());
			}

			let candidates = find_tgdb_games_by_title_normalized(&q_norm, &db_conn).await?;
			let scored = score_local_candidates(&parsed_dat, &candidates);
			if try_write_cross(
				&db_conn,
				&mut redis_conn,
				&game,
				&sibling,
				record_pick_best(
					"thegamesdb",
					"cross_normalized",
					pick_best(scored.iter().map(|s| (s, s.score))),
				),
				AutomaticMatchReasonEnum::CrossProviderNormalizedName,
				"Normalized",
			)
			.await?
			{
				return Ok(());
			}

			let candidates = find_tgdb_games_by_alias_name_normalized(&q_norm, &db_conn).await?;
			let scored = score_local_candidates(&parsed_dat, &candidates);
			if try_write_cross(
				&db_conn,
				&mut redis_conn,
				&game,
				&sibling,
				record_pick_best(
					"thegamesdb",
					"cross_normalized_alternative",
					pick_best(scored.iter().map(|s| (s, s.score))),
				),
				AutomaticMatchReasonEnum::CrossProviderNormalizedName,
				"Normalized alt",
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
	redis_conn: &mut MultiplexedConnection,
	game: &Model,
	sibling: &str,
	selection: Selection<'_, ScoredCand<'_>>,
	reason: AutomaticMatchReasonEnum,
	label: &str,
) -> anyhow::Result<bool> {
	match selection {
		Selection::Best(s) => {
			debug!(
				"Cross-matched Game \"{}\" to TheGamesDB Game ID {} via sibling \"{}\" ({label})",
				&game.name, s.cand.id, sibling
			);
			write_auto_match_success(
				"thegamesdb",
				MetadataProviderEnum::TheGamesDB,
				Target::Game(game.id),
				s.cand.id.to_string(),
				reason,
				Some(s.cand.title.clone()),
				None,
				db_conn,
				redis_conn,
			)
			.await?;
			Ok(true)
		}
		Selection::Ambiguous => {
			write_auto_match_failed(
				"thegamesdb",
				MetadataProviderEnum::TheGamesDB,
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
