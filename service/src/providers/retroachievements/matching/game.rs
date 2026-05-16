use crate::db::game::{
	get_automatic_match_failed_games_with_limit, get_unmatched_games_with_clone_of_with_limit,
	get_unmatched_games_without_clone_of_with_limit,
};
use crate::db::game_file::get_game_files_from_game_id;
use crate::db::platform::{
	find_platform_of_game, find_platform_related_signature_metadata_mapping,
};
use crate::db::retroachievements::{
	find_retroachievements_game_by_md5, find_retroachievements_games_by_system_and_title,
	find_retroachievements_games_by_system_and_title_normalized,
};
use crate::matching::name_parse::{ParsedName, parse_name};
use crate::matching::scoring::{
	CandidateGate, CandidateScore, Selection, gate_and_score, pick_best, record_pick_best,
};
use crate::matching::util::normalize_title;
use crate::providers::retroachievements::RetroAchievementsClient;
use crate::providers::{
	DEFAULT_CHUNK_SIZE, MetadataProvider, Target, drive_match_pipeline, write_auto_match_failed,
	write_auto_match_success,
};
use entity::game::Model;
use entity::retroachievements_game;
use entity::sea_orm_active_enums::{
	AutomaticMatchReasonEnum, FailedMatchReasonEnum, MatchTypeEnum, MetadataProviderEnum,
};
use futures_util::future::BoxFuture;
use log::debug;
use redis::aio::MultiplexedConnection;
use sea_orm::DbConn;
use std::sync::Arc;

pub async fn match_games_to_retroachievements(
	client: Arc<RetroAchievementsClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	drive_match_pipeline(
		"game",
		MetadataProviderEnum::RetroAchievements,
		get_unmatched_games_without_clone_of_with_limit,
		match_game_to_retroachievements,
		client.clone(),
		db_conn,
		DEFAULT_CHUNK_SIZE,
	)
	.await?;
	debug!("Finished matching games without clone_of id to RetroAchievements");

	drive_match_pipeline(
		"game",
		MetadataProviderEnum::RetroAchievements,
		get_unmatched_games_with_clone_of_with_limit,
		match_clone_of_game_to_retroachievements,
		client.clone(),
		db_conn,
		DEFAULT_CHUNK_SIZE,
	)
	.await?;
	debug!("Finished matching games with clone_of id to RetroAchievements");

	drive_match_pipeline(
		"game",
		MetadataProviderEnum::RetroAchievements,
		get_automatic_match_failed_games_with_limit,
		match_game_to_retroachievements,
		client,
		db_conn,
		DEFAULT_CHUNK_SIZE,
	)
	.await?;
	debug!("Finished retrying previously failed RetroAchievements game matches");

	Ok(())
}

fn match_clone_of_game_to_retroachievements(
	game: Model,
	client: Arc<RetroAchievementsClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		crate::providers::drive_clone_propagation(
			game,
			client,
			db_conn,
			match_game_to_retroachievements,
		)
		.await
	})
}

fn match_game_to_retroachievements(
	game: Model,
	client: Arc<RetroAchievementsClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		let mut redis_conn = client.redis_conn().clone();

		if let Some(()) = try_match_by_md5(&game, &db_conn, &mut redis_conn).await? {
			return Ok(());
		}

		// Name fallback. Skip cleanly when the platform has no RA mapping
		// yet; the game will be retried after the platform pass writes one
		// on a future cycle.
		let system_name = match resolve_ra_system_name(&game, &db_conn).await? {
			Some(name) => name,
			None => {
				debug!(
					"Skipping RetroAchievements name fallback for Game \"{}\": platform has no RA mapping",
					game.name
				);
				return Ok(());
			}
		};

		let parsed_dat = parse_name(&game.name);
		let cleaned = parsed_dat.base.to_lowercase();
		let cleaned_normalized = normalize_title(&cleaned);

		let candidates =
			find_retroachievements_games_by_system_and_title(&system_name, &cleaned, &db_conn)
				.await?;
		let scored = score_ra_candidates(&parsed_dat, &candidates);
		if let Some(()) = run_ra_rung(
			&db_conn,
			&mut redis_conn,
			&game,
			record_pick_best(
				"retroachievements",
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

		let candidates = find_retroachievements_games_by_system_and_title_normalized(
			&system_name,
			&cleaned_normalized,
			&db_conn,
		)
		.await?;
		let scored = score_ra_candidates(&parsed_dat, &candidates);
		if let Some(()) = run_ra_rung(
			&db_conn,
			&mut redis_conn,
			&game,
			record_pick_best(
				"retroachievements",
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

		debug!(
			"No RetroAchievements match found for Game \"{}\"",
			&game.name
		);
		write_auto_match_failed(
			"retroachievements",
			MetadataProviderEnum::RetroAchievements,
			Target::Game(game.id),
			FailedMatchReasonEnum::NoDirectMatch,
			&db_conn,
			&mut redis_conn,
		)
		.await?;

		Ok(())
	})
}

async fn try_match_by_md5(
	game: &Model,
	db_conn: &DbConn,
	redis_conn: &mut MultiplexedConnection,
) -> anyhow::Result<Option<()>> {
	let files = get_game_files_from_game_id(game.id, db_conn).await?;
	for file in &files {
		if let Some(md5) = file.md5.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
			let hit = match find_retroachievements_game_by_md5(md5, db_conn).await? {
				Some(found) => {
					debug!(
						"Matched Game \"{}\" to RetroAchievements Game ID {} (MD5)",
						game.name, found.game_id
					);
					write_auto_match_success(
						"retroachievements",
						MetadataProviderEnum::RetroAchievements,
						Target::Game(game.id),
						found.game_id.to_string(),
						AutomaticMatchReasonEnum::Md5Hash,
						Some(found.title),
						None,
						db_conn,
						redis_conn,
					)
					.await?;
					true
				}
				None => false,
			};
			crate::metrics::record_match_rung(
				"retroachievements",
				"md5_hash",
				if hit { "hit" } else { "miss" },
			);
			if hit {
				return Ok(Some(()));
			}
		}
	}
	Ok(None)
}

/// Looks up the playmatch platform's RA mapping and returns the RA system
/// name (stored in `matched_name` by the platform pass). Returns `None` when
/// no RA mapping exists yet so the caller can defer cleanly without writing
/// a `Failed` row.
async fn resolve_ra_system_name(game: &Model, db_conn: &DbConn) -> anyhow::Result<Option<String>> {
	let Some(platform) = find_platform_of_game(game.id, db_conn).await? else {
		return Ok(None);
	};
	let mapping = find_platform_related_signature_metadata_mapping(
		&platform,
		MetadataProviderEnum::RetroAchievements,
		db_conn,
	)
	.await?;
	let Some(mapping) = mapping else {
		return Ok(None);
	};
	if !matches!(
		mapping.match_type,
		MatchTypeEnum::Automatic | MatchTypeEnum::Manual
	) {
		return Ok(None);
	}
	Ok(mapping.matched_name)
}

#[derive(Clone)]
struct ScoredCand<'a> {
	cand: &'a retroachievements_game::Model,
	score: CandidateScore,
}

fn score_ra_candidates<'a>(
	parsed_dat: &ParsedName,
	candidates: &'a [retroachievements_game::Model],
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

async fn run_ra_rung(
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
				"Matched Game \"{}\" to RetroAchievements Game ID {} ({label})",
				game.name, s.cand.game_id
			);
			write_auto_match_success(
				"retroachievements",
				MetadataProviderEnum::RetroAchievements,
				Target::Game(game.id),
				s.cand.game_id.to_string(),
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
			debug!(
				"Refusing to match Game \"{}\" on RetroAchievements: tied at top of score",
				game.name
			);
			write_auto_match_failed(
				"retroachievements",
				MetadataProviderEnum::RetroAchievements,
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
