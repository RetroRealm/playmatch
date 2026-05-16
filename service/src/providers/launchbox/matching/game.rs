use crate::db::game::{
	get_automatic_match_failed_games_with_limit, get_unmatched_games_with_clone_of_with_limit,
	get_unmatched_games_without_clone_of_with_limit,
};
use crate::db::launchbox::{
	find_lb_games_by_platform_and_alternate_name_region_priority,
	find_lb_games_by_platform_and_alternate_normalized_name_region_priority,
	find_lb_games_by_platform_and_name, find_lb_games_by_platform_and_normalized_name,
};
use crate::db::platform::{
	find_platform_of_game, find_platform_related_signature_metadata_mapping,
};
use crate::matching::name_parse::{ParsedName, parse_name};
use crate::matching::scoring::{
	CandidateGate, CandidateScore, Selection, gate_and_score, pick_best, record_pick_best,
};
use crate::matching::util::{clean_name, normalize_title};
use crate::providers::MetadataProvider;
use crate::providers::launchbox::LaunchBoxClient;
use crate::providers::{
	Target, drive_match_pipeline, write_auto_match_failed, write_auto_match_success,
};
use entity::game::Model;
use entity::launchbox_game;
use entity::sea_orm_active_enums::{
	AutomaticMatchReasonEnum, FailedMatchReasonEnum, MatchTypeEnum, MetadataProviderEnum,
};
use futures_util::future::BoxFuture;
use log::debug;
use redis::aio::MultiplexedConnection;
use sea_orm::DbConn;
use std::sync::Arc;

pub async fn match_games_to_launchbox(
	client: Arc<LaunchBoxClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	let chunk_size = client.chunk_size();
	drive_match_pipeline(
		"game",
		MetadataProviderEnum::Launchbox,
		get_unmatched_games_without_clone_of_with_limit,
		match_game_to_launchbox,
		client.clone(),
		db_conn,
		chunk_size,
	)
	.await?;
	debug!("Finished matching games without clone_of id to LaunchBox");

	drive_match_pipeline(
		"game",
		MetadataProviderEnum::Launchbox,
		get_unmatched_games_with_clone_of_with_limit,
		match_clone_of_game_to_launchbox,
		client.clone(),
		db_conn,
		chunk_size,
	)
	.await?;
	debug!("Finished matching games with clone_of id to LaunchBox");

	drive_match_pipeline(
		"game",
		MetadataProviderEnum::Launchbox,
		get_automatic_match_failed_games_with_limit,
		match_game_to_launchbox,
		client,
		db_conn,
		chunk_size,
	)
	.await?;
	debug!("Finished retrying previously failed LaunchBox game matches");

	Ok(())
}

fn match_clone_of_game_to_launchbox(
	game: Model,
	client: Arc<LaunchBoxClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(crate::providers::drive_clone_propagation(
		game,
		client,
		db_conn,
		match_game_to_launchbox,
	))
}

fn lb_year(found: &launchbox_game::Model) -> Option<i16> {
	found.release_year.and_then(|y| i16::try_from(y).ok())
}

#[derive(Clone)]
struct ScoredCand<'a> {
	cand: &'a launchbox_game::Model,
	score: CandidateScore,
}

/// Score every candidate via `gate_and_score`, drop the rejects.
fn score_lb_candidates<'a>(
	parsed_dat: &ParsedName,
	candidates: &'a [launchbox_game::Model],
) -> Vec<ScoredCand<'a>> {
	candidates
		.iter()
		.filter_map(|c| {
			let parsed_cand = parse_name(&c.name);
			let candidate_year = c.release_year.and_then(|y| u16::try_from(y).ok());
			match gate_and_score(
				parsed_dat,
				Some(&parsed_cand),
				candidate_year,
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

fn match_game_to_launchbox(
	game: Model,
	client: Arc<LaunchBoxClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		let mut redis_conn = client.redis_conn().clone();
		let platform_name = get_game_platform_launchbox_name(&game, &db_conn).await?;

		let parsed_dat = parse_name(&game.name);
		let cleaned = parsed_dat.base.to_lowercase();
		let cleaned_normalized = normalize_title(&cleaned);
		let dat_lb_regions: Vec<&'static str> = parsed_dat
			.regions
			.iter()
			.flat_map(|r| r.lb_codes().iter().copied())
			.collect();

		let candidates =
			find_lb_games_by_platform_and_name(&platform_name, &cleaned, &db_conn).await?;
		let scored = score_lb_candidates(&parsed_dat, &candidates);
		if let Some(()) = run_lb_rung(
			&db_conn,
			&mut redis_conn,
			&game,
			record_pick_best(
				"launchbox",
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

		let candidates = find_lb_games_by_platform_and_alternate_name_region_priority(
			&platform_name,
			&cleaned,
			&dat_lb_regions,
			&db_conn,
		)
		.await?;
		let scored = score_lb_candidates(&parsed_dat, &candidates);
		if let Some(()) = run_lb_rung(
			&db_conn,
			&mut redis_conn,
			&game,
			record_pick_best(
				"launchbox",
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

		let candidates = find_lb_games_by_platform_and_normalized_name(
			&platform_name,
			&cleaned_normalized,
			&db_conn,
		)
		.await?;
		let scored = score_lb_candidates(&parsed_dat, &candidates);
		if let Some(()) = run_lb_rung(
			&db_conn,
			&mut redis_conn,
			&game,
			record_pick_best(
				"launchbox",
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

		let candidates = find_lb_games_by_platform_and_alternate_normalized_name_region_priority(
			&platform_name,
			&cleaned_normalized,
			&dat_lb_regions,
			&db_conn,
		)
		.await?;
		let scored = score_lb_candidates(&parsed_dat, &candidates);
		if let Some(()) = run_lb_rung(
			&db_conn,
			&mut redis_conn,
			&game,
			record_pick_best(
				"launchbox",
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

		debug!("No LaunchBox match found for Game \"{}\"", &cleaned);
		write_auto_match_failed(
			"launchbox",
			MetadataProviderEnum::Launchbox,
			Target::Game(game.id),
			FailedMatchReasonEnum::NoDirectMatch,
			&db_conn,
			&mut redis_conn,
		)
		.await?;

		Ok(())
	})
}

async fn run_lb_rung(
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
				"Matched Game \"{}\" to LaunchBox Game ID {} ({label})",
				&game.name, s.cand.database_id
			);
			write_auto_match_success(
				"launchbox",
				MetadataProviderEnum::Launchbox,
				Target::Game(game.id),
				s.cand.database_id.to_string(),
				reason,
				Some(s.cand.name.clone()),
				lb_year(s.cand),
				db_conn,
				redis_conn,
			)
			.await?;
			Ok(Some(()))
		}
		Selection::Ambiguous => {
			debug!(
				"Refusing to match Game \"{}\" on LaunchBox: tied at top of score",
				&game.name
			);
			write_auto_match_failed(
				"launchbox",
				MetadataProviderEnum::Launchbox,
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

async fn get_game_platform_launchbox_name(
	game: &Model,
	db_conn: &DbConn,
) -> anyhow::Result<String> {
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
		MetadataProviderEnum::Launchbox,
		db_conn,
	)
	.await?
	.ok_or_else(|| {
		anyhow::anyhow!(
			"Platform {} is missing its launchbox metadata mapping, this shouldn't happen...",
			&platform.name
		)
	})?;

	if !matches!(
		mapping.match_type,
		MatchTypeEnum::Automatic | MatchTypeEnum::Manual
	) {
		return Err(anyhow::anyhow!(
			"Platform {} is not matched to LaunchBox, this shouldn't happen...",
			&platform.name
		));
	}

	mapping.provider_id.ok_or_else(|| {
		anyhow::anyhow!(
			"Platform {} launchbox mapping has no provider_id",
			&platform.name
		)
	})
}

pub fn match_game_via_sibling_name_launchbox(
	game: Model,
	sibling_names: Vec<String>,
	client: Arc<LaunchBoxClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		let mut redis_conn = client.redis_conn().clone();
		let platform_name = get_game_platform_launchbox_name(&game, &db_conn).await?;
		let parsed_dat = parse_name(&game.name);
		let cleaned_playmatch = parsed_dat.base.to_lowercase();
		let normalized_playmatch = normalize_title(&cleaned_playmatch);
		let dat_lb_regions: Vec<&'static str> = parsed_dat
			.regions
			.iter()
			.flat_map(|r| r.lb_codes().iter().copied())
			.collect();
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

			let candidates =
				find_lb_games_by_platform_and_name(&platform_name, &q, &db_conn).await?;
			let scored = score_lb_candidates(&parsed_dat, &candidates);
			if try_write_lb_cross(
				&db_conn,
				&mut redis_conn,
				&game,
				&sibling,
				record_pick_best(
					"launchbox",
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

			let candidates = find_lb_games_by_platform_and_alternate_name_region_priority(
				&platform_name,
				&q,
				&dat_lb_regions,
				&db_conn,
			)
			.await?;
			let scored = score_lb_candidates(&parsed_dat, &candidates);
			if try_write_lb_cross(
				&db_conn,
				&mut redis_conn,
				&game,
				&sibling,
				record_pick_best(
					"launchbox",
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

			let candidates =
				find_lb_games_by_platform_and_normalized_name(&platform_name, &q_norm, &db_conn)
					.await?;
			let scored = score_lb_candidates(&parsed_dat, &candidates);
			if try_write_lb_cross(
				&db_conn,
				&mut redis_conn,
				&game,
				&sibling,
				record_pick_best(
					"launchbox",
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

			let candidates =
				find_lb_games_by_platform_and_alternate_normalized_name_region_priority(
					&platform_name,
					&q_norm,
					&dat_lb_regions,
					&db_conn,
				)
				.await?;
			let scored = score_lb_candidates(&parsed_dat, &candidates);
			if try_write_lb_cross(
				&db_conn,
				&mut redis_conn,
				&game,
				&sibling,
				record_pick_best(
					"launchbox",
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

async fn try_write_lb_cross(
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
				"Cross-matched Game \"{}\" to LaunchBox Game ID {} via sibling \"{}\" ({label})",
				&game.name, s.cand.database_id, sibling
			);
			write_auto_match_success(
				"launchbox",
				MetadataProviderEnum::Launchbox,
				Target::Game(game.id),
				s.cand.database_id.to_string(),
				reason,
				Some(s.cand.name.clone()),
				lb_year(s.cand),
				db_conn,
				redis_conn,
			)
			.await?;
			Ok(true)
		}
		Selection::Ambiguous => {
			write_auto_match_failed(
				"launchbox",
				MetadataProviderEnum::Launchbox,
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
	use chrono::Utc;
	use sea_orm::prelude::Uuid;

	fn lb_candidate(
		database_id: i64,
		name: &str,
		release_year: Option<i32>,
	) -> launchbox_game::Model {
		launchbox_game::Model {
			id: Uuid::nil(),
			database_id,
			name: name.to_string(),
			name_normalized: Some(normalize_title(&name.to_lowercase())),
			platform_name: "Nintendo DS".to_string(),
			release_date: None,
			release_year,
			overview: None,
			developer: None,
			publisher: None,
			genres: None,
			max_players: None,
			cooperative: None,
			esrb: None,
			release_type: None,
			status: None,
			wikipedia_url: None,
			video_url: None,
			community_rating: None,
			community_rating_count: None,
			created_at: Utc::now().fixed_offset(),
			updated_at: Utc::now().fixed_offset(),
		}
	}

	#[test]
	fn score_lb_candidates_picks_year_match_over_unknown() {
		let parsed_dat = parse_name("Pokemon Diamond Version (USA) (2006)");
		let candidates = vec![
			lb_candidate(1, "Pokemon Diamond Version", None),
			lb_candidate(2, "Pokemon Diamond Version", Some(2006)),
		];
		let scored = score_lb_candidates(&parsed_dat, &candidates);
		let selection = pick_best(scored.iter().map(|s| (s, s.score)));
		match selection {
			Selection::Best(s) => assert_eq!(s.cand.database_id, 2),
			Selection::Ambiguous => panic!("expected Best, got Ambiguous"),
			Selection::None => panic!("expected Best, got None"),
		}
	}

	#[test]
	fn score_lb_candidates_returns_ambiguous_on_tied_top() {
		let parsed_dat = parse_name("Pokemon Diamond Version (USA)");
		let candidates = vec![
			lb_candidate(10, "Pokemon Diamond Version", None),
			lb_candidate(20, "Pokemon Diamond Version", None),
		];
		let scored = score_lb_candidates(&parsed_dat, &candidates);
		let selection = pick_best(scored.iter().map(|s| (s, s.score)));
		assert!(matches!(selection, Selection::Ambiguous));
	}

	#[test]
	fn score_lb_candidates_returns_none_when_year_two_off() {
		let parsed_dat = parse_name("Pokemon Diamond Version (USA) (2006)");
		let candidates = vec![lb_candidate(99, "Pokemon Diamond Version", Some(2010))];
		let scored = score_lb_candidates(&parsed_dat, &candidates);
		let selection = pick_best(scored.iter().map(|s| (s, s.score)));
		assert!(matches!(selection, Selection::None));
	}
}
