use crate::db::game::{
	find_game_parent, find_game_signature_metadata_mapping,
	get_automatic_match_failed_games_with_limit, get_unmatched_games_with_clone_of_with_limit,
	get_unmatched_games_without_clone_of_with_limit,
};
use crate::db::platform::{
	find_platform_of_game, find_platform_related_signature_metadata_mapping,
};
use crate::matching::name_parse::parse_name;
use crate::matching::scoring::{
	CandidateGate, CandidateScore, Selection, gate_and_score, pick_best,
};
use crate::matching::util::{clean_name, normalize_title};
use crate::providers::MetadataProvider;
use crate::providers::mobygames::MobyGamesClient;
use crate::providers::mobygames::model::MgGame;
use crate::providers::{
	Target, drive_match_pipeline, write_auto_match_failed, write_auto_match_success,
};
use entity::game::Model;
use entity::sea_orm_active_enums::{
	AutomaticMatchReasonEnum, FailedMatchReasonEnum, MatchTypeEnum, MetadataProviderEnum,
};
use futures_util::future::BoxFuture;
use log::debug;
use sea_orm::DbConn;
use std::sync::Arc;

pub async fn match_games_to_mobygames(
	client: Arc<MobyGamesClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	let chunk_size = client.chunk_size();
	drive_match_pipeline(
		"game",
		MetadataProviderEnum::Mobygames,
		get_unmatched_games_without_clone_of_with_limit,
		match_game_to_mobygames,
		client.clone(),
		db_conn,
		chunk_size,
	)
	.await?;
	debug!("Finished matching games without clone_of id to MobyGames");

	drive_match_pipeline(
		"game",
		MetadataProviderEnum::Mobygames,
		get_unmatched_games_with_clone_of_with_limit,
		match_clone_of_game_to_mobygames,
		client.clone(),
		db_conn,
		chunk_size,
	)
	.await?;
	debug!("Finished matching games with clone_of id to MobyGames");

	drive_match_pipeline(
		"game",
		MetadataProviderEnum::Mobygames,
		get_automatic_match_failed_games_with_limit,
		match_game_to_mobygames,
		client,
		db_conn,
		chunk_size,
	)
	.await?;
	debug!("Finished retrying previously failed MobyGames game matches");

	Ok(())
}

fn match_clone_of_game_to_mobygames(
	game: Model,
	client: Arc<MobyGamesClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		let mut redis_conn = client.redis_conn().clone();
		let parent_game = find_game_parent(&game, &db_conn).await?;

		if let Some(parent_game) = parent_game {
			let parent_mapping =
				find_game_signature_metadata_mapping(&parent_game, &db_conn).await?;

			if let Some(mapping) = &parent_mapping
				&& matches!(
					mapping.match_type,
					MatchTypeEnum::Automatic | MatchTypeEnum::Manual
				) && let Some(provider_id) = mapping.provider_id.clone()
			{
				debug!(
					"Matched Game \"{}\" to MobyGames Game ID {provider_id} (Via Parent)",
					&game.name
				);
				write_auto_match_success(
					"mobygames",
					MetadataProviderEnum::Mobygames,
					Target::Game(game.id),
					provider_id,
					AutomaticMatchReasonEnum::ViaParent,
					mapping.matched_name.clone(),
					mapping.matched_year,
					&db_conn,
					&mut redis_conn,
				)
				.await?;
				return Ok(());
			}

			match_game_to_mobygames(game.clone(), client.clone(), db_conn.clone()).await?;

			let mapping = find_game_signature_metadata_mapping(&game, &db_conn).await?;

			if let Some(mapping) = mapping
				&& matches!(
					mapping.match_type,
					MatchTypeEnum::Automatic | MatchTypeEnum::Manual
				) && let Some(provider_id) = mapping.provider_id
			{
				debug!("Propagating MobyGames match from clone to parent game (Via Child)");
				write_auto_match_success(
					"mobygames",
					MetadataProviderEnum::Mobygames,
					Target::Game(parent_game.id),
					provider_id,
					AutomaticMatchReasonEnum::ViaChild,
					mapping.matched_name,
					mapping.matched_year,
					&db_conn,
					&mut redis_conn,
				)
				.await?;
			}
		}

		Ok(())
	})
}

struct ScoredCand<'a> {
	cand: &'a MgGame,
	score: CandidateScore,
	year: Option<i16>,
}

fn match_game_to_mobygames(
	game: Model,
	client: Arc<MobyGamesClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		let mut redis_conn = client.redis_conn().clone();
		let platform_id = get_game_platform_mobygames_id(&game, &db_conn).await?;

		let parsed_dat = parse_name(&game.name);
		let cleaned = parsed_dat.base.to_lowercase();
		let cleaned_normalized = normalize_title(&cleaned);

		let candidates = client.search_games(Some(platform_id), &cleaned).await?;

		let mut direct: Vec<ScoredCand<'_>> = vec![];
		let mut normalized: Vec<ScoredCand<'_>> = vec![];
		let mut alt_direct: Vec<ScoredCand<'_>> = vec![];
		let mut alt_normalized: Vec<ScoredCand<'_>> = vec![];

		for c in &candidates {
			let candidate_year = mg_year_for_platform(c, platform_id);
			let candidate_platforms_i64 = mg_platforms_i64(c);
			let parsed_cand = parse_name(&c.title);
			let gate = gate_and_score(
				&parsed_dat,
				Some(&parsed_cand),
				candidate_year,
				&[],
				&[],
				candidate_platforms_i64.as_deref(),
				Some(platform_id),
				&[],
			);
			let score = match gate {
				CandidateGate::Reject => continue,
				CandidateGate::Pass(s) => s,
			};
			let year_i16 = candidate_year.and_then(|y| i16::try_from(y).ok());

			let main_lower = c.title.to_lowercase();
			if main_lower == cleaned {
				direct.push(ScoredCand {
					cand: c,
					score,
					year: year_i16,
				});
				continue;
			}
			if normalize_title(&main_lower) == cleaned_normalized {
				normalized.push(ScoredCand {
					cand: c,
					score,
					year: year_i16,
				});
				continue;
			}

			if let Some(alts) = c.alternate_titles.as_ref() {
				let mut placed = false;
				for alt in alts {
					let alt_lower = alt.title.to_lowercase();
					if alt_lower == cleaned {
						alt_direct.push(ScoredCand {
							cand: c,
							score,
							year: year_i16,
						});
						placed = true;
						break;
					}
				}
				if placed {
					continue;
				}
				for alt in alts {
					let alt_lower = alt.title.to_lowercase();
					if normalize_title(&alt_lower) == cleaned_normalized {
						alt_normalized.push(ScoredCand {
							cand: c,
							score,
							year: year_i16,
						});
						break;
					}
				}
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
		if let Some(()) = run_rung(
			&db_conn,
			&mut redis_conn,
			&game,
			pick_best(alt_direct.iter().map(|s| (s, s.score))),
			AutomaticMatchReasonEnum::AlternativeName,
			"Alternative Name Match",
		)
		.await?
		{
			return Ok(());
		}
		if let Some(()) = run_rung(
			&db_conn,
			&mut redis_conn,
			&game,
			pick_best(alt_normalized.iter().map(|s| (s, s.score))),
			AutomaticMatchReasonEnum::NormalizedAlternativeName,
			"Normalized Alternative Match",
		)
		.await?
		{
			return Ok(());
		}

		debug!("No MobyGames match found for Game \"{}\"", &cleaned);
		write_auto_match_failed(
			"mobygames",
			MetadataProviderEnum::Mobygames,
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
				"Matched Game \"{}\" to MobyGames Game ID {} ({label})",
				&game.name, s.cand.game_id
			);
			write_auto_match_success(
				"mobygames",
				MetadataProviderEnum::Mobygames,
				Target::Game(game.id),
				s.cand.game_id.to_string(),
				reason,
				Some(s.cand.title.clone()),
				s.year,
				db_conn,
				redis_conn,
			)
			.await?;
			Ok(Some(()))
		}
		Selection::Ambiguous => {
			debug!(
				"Refusing to match Game \"{}\" on MobyGames: tied at top of score",
				&game.name
			);
			write_auto_match_failed(
				"mobygames",
				MetadataProviderEnum::Mobygames,
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

async fn get_game_platform_mobygames_id(game: &Model, db_conn: &DbConn) -> anyhow::Result<i64> {
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
		MetadataProviderEnum::Mobygames,
		db_conn,
	)
	.await?
	.ok_or_else(|| {
		anyhow::anyhow!(
			"Platform {} is missing its mobygames metadata mapping, this shouldn't happen...",
			&platform.name
		)
	})?;

	if !matches!(
		mapping.match_type,
		MatchTypeEnum::Automatic | MatchTypeEnum::Manual
	) {
		return Err(anyhow::anyhow!(
			"Platform {} is not matched to MobyGames, this shouldn't happen...",
			&platform.name
		));
	}

	let provider_id = mapping.provider_id.ok_or_else(|| {
		anyhow::anyhow!(
			"Platform {} mobygames mapping has no provider_id",
			&platform.name
		)
	})?;
	provider_id.parse::<i64>().map_err(|e| {
		anyhow::anyhow!(
			"Platform {} mobygames provider_id {provider_id:?} is not a valid i64: {e}",
			&platform.name
		)
	})
}

pub fn match_game_via_sibling_name_mobygames(
	game: Model,
	sibling_names: Vec<String>,
	client: Arc<MobyGamesClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		let mut redis_conn = client.redis_conn().clone();
		let platform_id = get_game_platform_mobygames_id(&game, &db_conn).await?;
		let parsed_dat = parse_name(&game.name);
		let cleaned_playmatch = parsed_dat.base.to_lowercase();
		let mut tried: std::collections::HashSet<String> = std::collections::HashSet::new();
		tried.insert(cleaned_playmatch);

		for sibling in sibling_names {
			let q = clean_name(&sibling).to_lowercase();
			if !tried.insert(q.clone()) {
				continue;
			}
			let q_norm = normalize_title(&q);
			let candidates = client.search_games(Some(platform_id), &q).await?;

			let mut direct: Vec<ScoredCand<'_>> = vec![];
			let mut normalized: Vec<ScoredCand<'_>> = vec![];
			let mut alt_direct: Vec<ScoredCand<'_>> = vec![];
			let mut alt_normalized: Vec<ScoredCand<'_>> = vec![];

			for c in &candidates {
				let candidate_year = mg_year_for_platform(c, platform_id);
				let candidate_platforms_i64 = mg_platforms_i64(c);
				let parsed_cand = parse_name(&c.title);
				let gate = gate_and_score(
					&parsed_dat,
					Some(&parsed_cand),
					candidate_year,
					&[],
					&[],
					candidate_platforms_i64.as_deref(),
					Some(platform_id),
					&[],
				);
				let score = match gate {
					CandidateGate::Reject => continue,
					CandidateGate::Pass(s) => s,
				};
				let year_i16 = candidate_year.and_then(|y| i16::try_from(y).ok());

				let main_lower = c.title.to_lowercase();
				if main_lower == q {
					direct.push(ScoredCand {
						cand: c,
						score,
						year: year_i16,
					});
					continue;
				}
				if normalize_title(&main_lower) == q_norm {
					normalized.push(ScoredCand {
						cand: c,
						score,
						year: year_i16,
					});
					continue;
				}

				if let Some(alts) = c.alternate_titles.as_ref() {
					let mut placed = false;
					for alt in alts {
						let alt_lower = alt.title.to_lowercase();
						if alt_lower == q {
							alt_direct.push(ScoredCand {
								cand: c,
								score,
								year: year_i16,
							});
							placed = true;
							break;
						}
					}
					if placed {
						continue;
					}
					for alt in alts {
						let alt_lower = alt.title.to_lowercase();
						if normalize_title(&alt_lower) == q_norm {
							alt_normalized.push(ScoredCand {
								cand: c,
								score,
								year: year_i16,
							});
							break;
						}
					}
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
			if try_write_cross(
				&db_conn,
				&mut redis_conn,
				&game,
				&sibling,
				pick_best(alt_direct.iter().map(|s| (s, s.score))),
				AutomaticMatchReasonEnum::CrossProviderDirectName,
				"Alternative",
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
				pick_best(alt_normalized.iter().map(|s| (s, s.score))),
				AutomaticMatchReasonEnum::CrossProviderNormalizedName,
				"Normalized Alternative",
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
				"Cross-matched Game \"{}\" to MobyGames Game ID {} via sibling \"{}\" ({label})",
				&game.name, s.cand.game_id, sibling
			);
			write_auto_match_success(
				"mobygames",
				MetadataProviderEnum::Mobygames,
				Target::Game(game.id),
				s.cand.game_id.to_string(),
				reason,
				Some(s.cand.title.clone()),
				s.year,
				db_conn,
				redis_conn,
			)
			.await?;
			Ok(true)
		}
		Selection::Ambiguous => {
			debug!(
				"Refusing to cross-match Game \"{}\" on MobyGames: tied at top of score",
				&game.name
			);
			write_auto_match_failed(
				"mobygames",
				MetadataProviderEnum::Mobygames,
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

fn mg_year_for_platform(c: &MgGame, our_platform_id: i64) -> Option<u16> {
	c.platforms
		.as_ref()?
		.iter()
		.find(|p| p.platform_id == our_platform_id)
		.and_then(|p| p.first_release_date.as_ref())
		.and_then(|s| s.get(..4))
		.and_then(|y| y.parse::<u16>().ok())
}

fn mg_platforms_i64(c: &MgGame) -> Option<Vec<i64>> {
	c.platforms
		.as_ref()
		.map(|v| v.iter().map(|p| p.platform_id).collect())
}
