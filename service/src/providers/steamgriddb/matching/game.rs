use crate::db::game::{
	get_automatic_match_failed_games_with_limit,
	get_unmatched_games_with_clone_of_with_limit_no_platform_gate,
	get_unmatched_games_without_clone_of_with_limit_no_platform_gate,
};
use crate::matching::name_parse::{ParsedName, parse_name};
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
use redis::aio::MultiplexedConnection;
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

		if try_sgdb_query(
			&client,
			&db_conn,
			&mut redis_conn,
			&game,
			&parsed_dat,
			&cleaned,
			&cleaned_normalized,
			&cleaned,
		)
		.await?
		{
			return Ok(());
		}

		// SteamGridDB's autocomplete tokenises diacritics and punctuation
		// inconsistently. When the cleaned form returns nothing scoreable, retry
		// with the normalized form, which is what the local comparison already
		// requires anyway.
		if cleaned_normalized != cleaned
			&& try_sgdb_query(
				&client,
				&db_conn,
				&mut redis_conn,
				&game,
				&parsed_dat,
				&cleaned,
				&cleaned_normalized,
				&cleaned_normalized,
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

/// One SGDB search call followed by the direct + normalized rungs. Returns
/// `true` when either rung wrote a mapping (success or ambiguous), `false`
/// when both rungs missed so the caller can decide whether to retry with a
/// different query form.
#[allow(clippy::too_many_arguments)]
async fn try_sgdb_query(
	client: &SteamGridDbClient,
	db_conn: &DbConn,
	redis_conn: &mut MultiplexedConnection,
	game: &Model,
	parsed_dat: &ParsedName,
	q: &str,
	q_norm: &str,
	search_term: &str,
) -> anyhow::Result<bool> {
	let candidates = client.search_games(search_term).await?;
	let (direct, normalized) = collect_sgdb_rungs(&candidates, parsed_dat, q, q_norm);

	if run_rung(
		db_conn,
		redis_conn,
		game,
		pick_best(direct.iter().map(|s| (s, s.score))),
		AutomaticMatchReasonEnum::DirectName,
		"Direct Match",
	)
	.await?
	.is_some()
	{
		return Ok(true);
	}
	if run_rung(
		db_conn,
		redis_conn,
		game,
		pick_best(normalized.iter().map(|s| (s, s.score))),
		AutomaticMatchReasonEnum::NormalizedName,
		"Normalized Match",
	)
	.await?
	.is_some()
	{
		return Ok(true);
	}
	Ok(false)
}

/// Score each candidate against `q` (lowercase exact) and `q_norm`
/// (normalize_title equality) and bucket into the direct / normalized rungs.
fn collect_sgdb_rungs<'a>(
	candidates: &'a [SgdbGame],
	parsed_dat: &ParsedName,
	q: &str,
	q_norm: &str,
) -> (Vec<ScoredCand<'a>>, Vec<ScoredCand<'a>>) {
	let mut direct: Vec<ScoredCand<'a>> = vec![];
	let mut normalized: Vec<ScoredCand<'a>> = vec![];

	for c in candidates {
		let parsed_cand = parse_name(&c.name);
		let gate = gate_and_score(
			parsed_dat,
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

	(direct, normalized)
}

async fn run_rung(
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
		let normalized_playmatch = normalize_title(&cleaned_playmatch);
		let mut tried: HashSet<String> = HashSet::new();
		tried.insert(cleaned_playmatch);
		tried.insert(normalized_playmatch);

		for sibling in sibling_names {
			let q = clean_name(&sibling).to_lowercase();
			if !tried.insert(q.clone()) {
				continue;
			}
			let q_norm = normalize_title(&q);

			if try_sgdb_sibling_query(
				&client,
				&db_conn,
				&mut redis_conn,
				&game,
				&sibling,
				&parsed_dat,
				&q,
				&q_norm,
				&q,
			)
			.await?
			{
				return Ok(());
			}

			if q_norm != q
				&& tried.insert(q_norm.clone())
				&& try_sgdb_sibling_query(
					&client,
					&db_conn,
					&mut redis_conn,
					&game,
					&sibling,
					&parsed_dat,
					&q,
					&q_norm,
					&q_norm,
				)
				.await?
			{
				return Ok(());
			}
		}
		Ok(())
	})
}

#[allow(clippy::too_many_arguments)]
async fn try_sgdb_sibling_query(
	client: &SteamGridDbClient,
	db_conn: &DbConn,
	redis_conn: &mut MultiplexedConnection,
	game: &Model,
	sibling: &str,
	parsed_dat: &ParsedName,
	q: &str,
	q_norm: &str,
	search_term: &str,
) -> anyhow::Result<bool> {
	let candidates = client.search_games(search_term).await?;
	let (direct, normalized) = collect_sgdb_rungs(&candidates, parsed_dat, q, q_norm);

	if try_write_cross(
		db_conn,
		redis_conn,
		game,
		sibling,
		pick_best(direct.iter().map(|s| (s, s.score))),
		AutomaticMatchReasonEnum::CrossProviderDirectName,
		"Direct",
	)
	.await?
	{
		return Ok(true);
	}
	if try_write_cross(
		db_conn,
		redis_conn,
		game,
		sibling,
		pick_best(normalized.iter().map(|s| (s, s.score))),
		AutomaticMatchReasonEnum::CrossProviderNormalizedName,
		"Normalized",
	)
	.await?
	{
		return Ok(true);
	}
	Ok(false)
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

#[cfg(test)]
mod tests {
	use super::*;

	fn sgdb_candidate(id: i64, name: &str) -> SgdbGame {
		SgdbGame {
			id,
			name: name.to_string(),
			types: vec![],
			verified: false,
		}
	}

	#[test]
	fn collect_sgdb_rungs_matches_normalized_with_diacritics() {
		let candidates = vec![sgdb_candidate(5245253, "Pokémon Diamond Version")];
		let parsed_dat = parse_name("Pokemon - Diamant-Edition (Germany)");
		let q = "pokémon: diamond version";
		let q_norm = normalize_title(q);

		let (direct, normalized) = collect_sgdb_rungs(&candidates, &parsed_dat, q, &q_norm);

		assert!(direct.is_empty(), "candidate should not land in direct");
		assert_eq!(normalized.len(), 1, "candidate should land in normalized");
		assert_eq!(normalized[0].cand.id, 5245253);
	}

	#[test]
	fn collect_sgdb_rungs_direct_hit() {
		let candidates = vec![sgdb_candidate(5245253, "Pokémon Diamond Version")];
		let parsed_dat = parse_name("Pokémon Diamond Version (USA)");
		let q = "pokémon diamond version";
		let q_norm = normalize_title(q);

		let (direct, _normalized) = collect_sgdb_rungs(&candidates, &parsed_dat, q, &q_norm);

		assert_eq!(direct.len(), 1);
		assert_eq!(direct[0].cand.id, 5245253);
	}

	#[test]
	fn collect_sgdb_rungs_skips_unrelated() {
		let candidates = vec![sgdb_candidate(1539, "Yo! Noid")];
		let parsed_dat = parse_name("Pokemon - Diamant-Edition (Germany)");
		let q = "pokémon: diamond version";
		let q_norm = normalize_title(q);

		let (direct, normalized) = collect_sgdb_rungs(&candidates, &parsed_dat, q, &q_norm);

		assert!(direct.is_empty());
		assert!(normalized.is_empty());
	}
}
