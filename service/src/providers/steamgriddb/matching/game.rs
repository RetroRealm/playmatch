use crate::db::game::{
	get_automatic_match_failed_games_with_limit,
	get_unmatched_games_with_clone_of_with_limit_no_platform_gate,
	get_unmatched_games_without_clone_of_with_limit_no_platform_gate,
};
use crate::matching::name_parse::parse_name;
use crate::matching::scoring::{
	CandidateGate, CandidateScore, Selection, gate_and_score, pick_best,
};
use crate::matching::util::{clean_name, normalize_title};
use crate::providers::MetadataProvider;
use crate::providers::steamgriddb::SteamGridDbClient;
use crate::providers::steamgriddb::model::SgdbGame;
use crate::providers::{
	DEFAULT_CHUNK_SIZE, Target, drive_match_pipeline, write_auto_match_failed,
	write_auto_match_success,
};
use entity::game::Model;
use entity::sea_orm_active_enums::{
	AutomaticMatchReasonEnum, FailedMatchReasonEnum, MetadataProviderEnum,
};
use futures_util::future::BoxFuture;
use log::debug;
use sea_orm::DbConn;
use std::collections::HashSet;
use std::sync::Arc;

pub async fn match_games_to_steamgriddb(
	client: Arc<SteamGridDbClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	drive_match_pipeline(
		"game",
		MetadataProviderEnum::Steamgriddb,
		get_unmatched_games_without_clone_of_with_limit_no_platform_gate,
		match_game_to_steamgriddb,
		client.clone(),
		db_conn,
		DEFAULT_CHUNK_SIZE,
	)
	.await?;
	debug!("Finished matching games without clone_of id to SteamGridDB");

	drive_match_pipeline(
		"game",
		MetadataProviderEnum::Steamgriddb,
		get_unmatched_games_with_clone_of_with_limit_no_platform_gate,
		match_clone_of_game_to_steamgriddb,
		client.clone(),
		db_conn,
		DEFAULT_CHUNK_SIZE,
	)
	.await?;
	debug!("Finished matching games with clone_of id to SteamGridDB");

	drive_match_pipeline(
		"game",
		MetadataProviderEnum::Steamgriddb,
		get_automatic_match_failed_games_with_limit,
		match_game_to_steamgriddb,
		client,
		db_conn,
		DEFAULT_CHUNK_SIZE,
	)
	.await?;
	debug!("Finished retrying previously failed SteamGridDB game matches");

	Ok(())
}

fn match_clone_of_game_to_steamgriddb(
	game: Model,
	client: Arc<SteamGridDbClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(crate::providers::drive_clone_propagation(
		game,
		client,
		db_conn,
		match_game_to_steamgriddb,
	))
}

struct ScoredCand<'a> {
	cand: &'a SgdbGame,
	score: CandidateScore,
}

fn match_game_to_steamgriddb(
	game: Model,
	client: Arc<SteamGridDbClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		let mut redis_conn = client.redis_conn().clone();
		let parsed_dat = parse_name(&game.name);
		let cleaned = parsed_dat.base.to_lowercase();
		let cleaned_normalized = normalize_title(&cleaned);

		let candidates = client.search_games(&cleaned).await?;

		let mut direct: Vec<ScoredCand<'_>> = vec![];
		let mut normalized: Vec<ScoredCand<'_>> = vec![];

		for c in &candidates {
			let parsed_cand = parse_name(&c.name);
			let gate = gate_and_score(
				&parsed_dat,
				Some(&parsed_cand),
				None,
				&[],
				&[],
				None,
				None,
				&[],
			);
			let score = match gate {
				CandidateGate::Reject => continue,
				CandidateGate::Pass(s) => s,
			};

			let lower = c.name.to_lowercase();
			if lower == cleaned {
				direct.push(ScoredCand { cand: c, score });
				continue;
			}
			if normalize_title(&lower) == cleaned_normalized {
				normalized.push(ScoredCand { cand: c, score });
			}
		}

		if let Some(()) = run_rung(
			&db_conn,
			&mut redis_conn,
			&game,
			pick_best(direct.iter().map(|s| (s, s.score))),
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
			pick_best(normalized.iter().map(|s| (s, s.score))),
			AutomaticMatchReasonEnum::NormalizedName,
			"Normalized Match",
		)
		.await?
		{
			return Ok(());
		}

		debug!("No SteamGridDB match found for Game \"{}\"", &cleaned);
		write_auto_match_failed(
			"steamgriddb",
			MetadataProviderEnum::Steamgriddb,
			Target::Game(game.id),
			FailedMatchReasonEnum::NoDirectMatch,
			&db_conn,
			&mut redis_conn,
		)
		.await?;

		Ok(())
	})
}

async fn run_rung(
	db_conn: &DbConn,
	redis_conn: &mut redis::aio::MultiplexedConnection,
	game: &Model,
	selection: Selection<'_, ScoredCand<'_>>,
	reason: AutomaticMatchReasonEnum,
	label: &str,
) -> anyhow::Result<Option<()>> {
	match selection {
		Selection::Best(s) => {
			debug!(
				"Matched Game \"{}\" to SteamGridDB Game ID {} ({label})",
				&game.name, s.cand.id
			);
			write_auto_match_success(
				"steamgriddb",
				MetadataProviderEnum::Steamgriddb,
				Target::Game(game.id),
				s.cand.id.to_string(),
				reason,
				Some(s.cand.name.clone()),
				None,
				db_conn,
				redis_conn,
			)
			.await?;
			Ok(Some(()))
		}
		Selection::Ambiguous => {
			write_auto_match_failed(
				"steamgriddb",
				MetadataProviderEnum::Steamgriddb,
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

pub fn match_game_via_sibling_name_steamgriddb(
	game: Model,
	sibling_names: Vec<String>,
	client: Arc<SteamGridDbClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		let mut redis_conn = client.redis_conn().clone();
		let parsed_dat = parse_name(&game.name);
		let cleaned_playmatch = parsed_dat.base.to_lowercase();
		let mut tried: HashSet<String> = HashSet::new();
		tried.insert(cleaned_playmatch);

		for sibling in sibling_names {
			let q = clean_name(&sibling).to_lowercase();
			if !tried.insert(q.clone()) {
				continue;
			}
			let q_norm = normalize_title(&q);
			let candidates = client.search_games(&q).await?;

			let mut direct: Vec<ScoredCand<'_>> = vec![];
			let mut normalized: Vec<ScoredCand<'_>> = vec![];

			for c in &candidates {
				let parsed_cand = parse_name(&c.name);
				let gate = gate_and_score(
					&parsed_dat,
					Some(&parsed_cand),
					None,
					&[],
					&[],
					None,
					None,
					&[],
				);
				let score = match gate {
					CandidateGate::Reject => continue,
					CandidateGate::Pass(s) => s,
				};

				let lower = c.name.to_lowercase();
				if lower == q {
					direct.push(ScoredCand { cand: c, score });
					continue;
				}
				if normalize_title(&lower) == q_norm {
					normalized.push(ScoredCand { cand: c, score });
				}
			}

			if try_write_cross(
				&db_conn,
				&mut redis_conn,
				&game,
				&sibling,
				pick_best(direct.iter().map(|s| (s, s.score))),
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
				pick_best(normalized.iter().map(|s| (s, s.score))),
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
	selection: Selection<'_, ScoredCand<'_>>,
	reason: AutomaticMatchReasonEnum,
	label: &str,
) -> anyhow::Result<bool> {
	match selection {
		Selection::Best(s) => {
			debug!(
				"Cross-matched Game \"{}\" to SteamGridDB Game ID {} via sibling \"{}\" ({label})",
				&game.name, s.cand.id, sibling
			);
			write_auto_match_success(
				"steamgriddb",
				MetadataProviderEnum::Steamgriddb,
				Target::Game(game.id),
				s.cand.id.to_string(),
				reason,
				Some(s.cand.name.clone()),
				None,
				db_conn,
				redis_conn,
			)
			.await?;
			Ok(true)
		}
		Selection::Ambiguous => {
			write_auto_match_failed(
				"steamgriddb",
				MetadataProviderEnum::Steamgriddb,
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
