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
use crate::providers::igdb::IgdbClient;
use crate::providers::igdb::model::Game as IgdbGame;
use crate::providers::{
	DEFAULT_CHUNK_SIZE, Target, drive_match_pipeline, write_auto_match_failed,
	write_auto_match_success,
};
use chrono::{DateTime, Datelike, Utc};
use entity::game::Model;
use entity::sea_orm_active_enums::{
	AutomaticMatchReasonEnum, FailedMatchReasonEnum, MatchTypeEnum, MetadataProviderEnum,
};
use futures_util::future::BoxFuture;
use log::debug;
use sea_orm::DbConn;
use std::sync::Arc;

pub async fn match_games_to_igdb(
	igdb_client: Arc<IgdbClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	drive_match_pipeline(
		"game",
		MetadataProviderEnum::Igdb,
		get_unmatched_games_without_clone_of_with_limit,
		match_game_to_igdb,
		igdb_client.clone(),
		db_conn,
		DEFAULT_CHUNK_SIZE,
	)
	.await?;
	debug!("Finished matching games without clone_of id to IGDB");

	drive_match_pipeline(
		"game",
		MetadataProviderEnum::Igdb,
		get_unmatched_games_with_clone_of_with_limit,
		match_clone_of_game_to_igdb,
		igdb_client.clone(),
		db_conn,
		DEFAULT_CHUNK_SIZE,
	)
	.await?;
	debug!("Finished matching games with clone_of id to IGDB");

	drive_match_pipeline(
		"game",
		MetadataProviderEnum::Igdb,
		get_automatic_match_failed_games_with_limit,
		match_game_to_igdb,
		igdb_client.clone(),
		db_conn,
		DEFAULT_CHUNK_SIZE,
	)
	.await?;
	debug!("Finished matching games which failed to match 60 days ago to IGDB");

	Ok(())
}

fn match_clone_of_game_to_igdb(
	game: Model,
	igdb_client: Arc<IgdbClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		let mut redis_conn = igdb_client.redis_conn().clone();
		let parent_game = find_game_parent(&game, &db_conn).await?;

		if let Some(parent_game) = parent_game {
			let parent_game_igdb_mapping =
				find_game_signature_metadata_mapping(&parent_game, &db_conn).await?;

			if let Some(parent_game_igdb_mapping) = &parent_game_igdb_mapping
				&& (parent_game_igdb_mapping.match_type == MatchTypeEnum::Automatic
					|| parent_game_igdb_mapping.match_type == MatchTypeEnum::Manual)
			{
				debug!(
					"Matched Game \"{}\" to IGDB Game ID {} (Via Parent)",
					&game.name,
					parent_game_igdb_mapping.provider_id.clone().unwrap()
				);

				write_auto_match_success(
					"igdb",
					MetadataProviderEnum::Igdb,
					Target::Game(game.id),
					parent_game_igdb_mapping.provider_id.clone().unwrap(),
					AutomaticMatchReasonEnum::ViaParent,
					parent_game_igdb_mapping.matched_name.clone(),
					parent_game_igdb_mapping.matched_year,
					&db_conn,
					&mut redis_conn,
				)
				.await?;

				return Ok(());
			}

			match_game_to_igdb(game.clone(), igdb_client.clone(), db_conn.clone()).await?;

			let mapping = find_game_signature_metadata_mapping(&game, &db_conn).await?;

			if let Some(mapping) = mapping
				&& (mapping.match_type == MatchTypeEnum::Automatic
					|| mapping.match_type == MatchTypeEnum::Manual)
			{
				debug!(
					"Matched Game with parent which is not matched, overriding parent mapping... (Via Child)"
				);

				write_auto_match_success(
					"igdb",
					MetadataProviderEnum::Igdb,
					Target::Game(parent_game.id),
					mapping.provider_id.unwrap(),
					AutomaticMatchReasonEnum::ViaChild,
					mapping.matched_name,
					mapping.matched_year,
					&db_conn,
					&mut redis_conn,
				)
				.await?;

				return Ok(());
			}
		}

		Ok(())
	})
}

struct ScoredCand<'a> {
	cand: &'a IgdbGame,
	score: CandidateScore,
	year: Option<i16>,
}

fn match_game_to_igdb(
	game: Model,
	igdb_client: Arc<IgdbClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		let mut redis_conn = igdb_client.redis_conn().clone();
		let platform_igdb_id = get_game_platform_igdb_id(&game, &db_conn).await?;

		let parsed_dat = parse_name(&game.name);
		let clean_name = parsed_dat.base.to_lowercase();
		let clean_name_normalized = normalize_title(&clean_name);

		let search_results = igdb_client
			.search_game_by_name_and_platform(&clean_name, platform_igdb_id)
			.await?;

		let mut direct: Vec<ScoredCand<'_>> = vec![];
		let mut normalized: Vec<ScoredCand<'_>> = vec![];
		let mut needs_alt: Vec<(&IgdbGame, CandidateScore, Option<i16>)> = vec![];

		for c in &search_results {
			let candidate_year = c.first_release_date.and_then(year_from_unix_seconds);
			let candidate_platforms_i64 = c
				.platforms
				.as_ref()
				.map(|v| v.iter().map(|p| *p as i64).collect::<Vec<_>>());
			let parsed_cand = parse_name(&c.name);
			let gate = gate_and_score(
				&parsed_dat,
				Some(&parsed_cand),
				candidate_year,
				&[],
				&[],
				candidate_platforms_i64.as_deref(),
				Some(platform_igdb_id as i64),
				&[],
			);
			let score = match gate {
				CandidateGate::Reject => continue,
				CandidateGate::Pass(s) => s,
			};
			let year_i16 = candidate_year.and_then(|y| i16::try_from(y).ok());

			let cand_lower = c.name.to_lowercase();
			if cand_lower == clean_name {
				direct.push(ScoredCand {
					cand: c,
					score,
					year: year_i16,
				});
				continue;
			}
			if normalize_title(&cand_lower) == clean_name_normalized {
				normalized.push(ScoredCand {
					cand: c,
					score,
					year: year_i16,
				});
				continue;
			}
			if c.alternative_names.is_some() {
				needs_alt.push((c, score, year_i16));
			}
		}

		match pick_best(direct.iter().map(|s| (s, s.score))) {
			Selection::Best(s) => {
				return write_match(
					&db_conn,
					&mut redis_conn,
					&game,
					s.cand,
					s.year,
					AutomaticMatchReasonEnum::DirectName,
					"Direct Match",
				)
				.await;
			}
			Selection::Ambiguous => {
				return write_ambiguous(&db_conn, &mut redis_conn, &game).await;
			}
			Selection::None => {}
		}

		match pick_best(normalized.iter().map(|s| (s, s.score))) {
			Selection::Best(s) => {
				return write_match(
					&db_conn,
					&mut redis_conn,
					&game,
					s.cand,
					s.year,
					AutomaticMatchReasonEnum::NormalizedName,
					"Normalized Name Match",
				)
				.await;
			}
			Selection::Ambiguous => {
				return write_ambiguous(&db_conn, &mut redis_conn, &game).await;
			}
			Selection::None => {}
		}

		let mut alt_direct: Vec<ScoredCand<'_>> = vec![];
		let mut alt_normalized: Vec<ScoredCand<'_>> = vec![];
		for (c, score, year_i16) in &needs_alt {
			let alt_ids = c.alternative_names.clone().expect("filtered above");
			let alt_resolved = igdb_client.get_alternative_names_by_id(alt_ids).await?;
			for alt in alt_resolved {
				let alt_lower = alt.name.to_lowercase();
				if alt_lower == clean_name {
					alt_direct.push(ScoredCand {
						cand: c,
						score: *score,
						year: *year_i16,
					});
					break;
				}
				if normalize_title(&alt_lower) == clean_name_normalized {
					alt_normalized.push(ScoredCand {
						cand: c,
						score: *score,
						year: *year_i16,
					});
					break;
				}
			}
		}

		match pick_best(alt_direct.iter().map(|s| (s, s.score))) {
			Selection::Best(s) => {
				return write_match(
					&db_conn,
					&mut redis_conn,
					&game,
					s.cand,
					s.year,
					AutomaticMatchReasonEnum::AlternativeName,
					"Alternative Name Match",
				)
				.await;
			}
			Selection::Ambiguous => {
				return write_ambiguous(&db_conn, &mut redis_conn, &game).await;
			}
			Selection::None => {}
		}

		match pick_best(alt_normalized.iter().map(|s| (s, s.score))) {
			Selection::Best(s) => {
				return write_match(
					&db_conn,
					&mut redis_conn,
					&game,
					s.cand,
					s.year,
					AutomaticMatchReasonEnum::NormalizedAlternativeName,
					"Normalized Alternative Name Match",
				)
				.await;
			}
			Selection::Ambiguous => {
				return write_ambiguous(&db_conn, &mut redis_conn, &game).await;
			}
			Selection::None => {}
		}

		debug!("No match found for Game \"{}\"", &clean_name);
		write_auto_match_failed(
			"igdb",
			MetadataProviderEnum::Igdb,
			Target::Game(game.id),
			FailedMatchReasonEnum::NoDirectMatch,
			&db_conn,
			&mut redis_conn,
		)
		.await?;

		Ok(())
	})
}

async fn write_match(
	db_conn: &DbConn,
	redis_conn: &mut redis::aio::MultiplexedConnection,
	game: &Model,
	cand: &IgdbGame,
	year: Option<i16>,
	reason: AutomaticMatchReasonEnum,
	label: &str,
) -> anyhow::Result<()> {
	debug!(
		"Matched Game \"{}\" to IGDB Game ID {} ({label})",
		&game.name, cand.id
	);
	write_auto_match_success(
		"igdb",
		MetadataProviderEnum::Igdb,
		Target::Game(game.id),
		cand.id.to_string(),
		reason,
		Some(cand.name.clone()),
		year,
		db_conn,
		redis_conn,
	)
	.await
}

async fn write_ambiguous(
	db_conn: &DbConn,
	redis_conn: &mut redis::aio::MultiplexedConnection,
	game: &Model,
) -> anyhow::Result<()> {
	debug!(
		"Refusing to match Game \"{}\" on IGDB: candidates tied at the top of the score",
		&game.name
	);
	write_auto_match_failed(
		"igdb",
		MetadataProviderEnum::Igdb,
		Target::Game(game.id),
		FailedMatchReasonEnum::Ambiguous,
		db_conn,
		redis_conn,
	)
	.await
}

async fn get_game_platform_igdb_id(game: &Model, db_conn: &DbConn) -> anyhow::Result<i32> {
	let platform = match find_platform_of_game(game.id, db_conn).await? {
		None => {
			return Err(anyhow::anyhow!(
				"No platform found for Game \"{}\", this shouldn't happen...",
				game.name
			));
		}
		Some(p) => p,
	};

	let platform_igdb_metadata_mapping = match find_platform_related_signature_metadata_mapping(
		&platform,
		MetadataProviderEnum::Igdb,
		db_conn,
	)
	.await?
	{
		None => {
			return Err(anyhow::anyhow!(
				"Platform {} is missing its igdb metadata mapping, this shouldn't happen...",
				&platform.name
			));
		}
		Some(plat_map) => plat_map,
	};

	if platform_igdb_metadata_mapping.match_type != MatchTypeEnum::Automatic
		&& platform_igdb_metadata_mapping.match_type != MatchTypeEnum::Manual
	{
		return Err(anyhow::anyhow!(
			"Platform {} is not matched to IGDB, this shouldn't happen...",
			&platform.name
		));
	}

	let raw = platform_igdb_metadata_mapping.provider_id.ok_or_else(|| {
		anyhow::anyhow!(
			"Platform {} is missing its igdb id on its metadata mapping, this shouldn't happen...",
			&platform.name
		)
	})?;

	raw.parse::<i32>().map_err(|e| {
		anyhow::anyhow!(
			"Platform {} has a non-numeric igdb provider_id ({raw}): {e}",
			&platform.name
		)
	})
}

pub fn match_game_via_sibling_name_igdb(
	game: Model,
	sibling_names: Vec<String>,
	igdb_client: Arc<IgdbClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		let mut redis_conn = igdb_client.redis_conn().clone();
		let platform_igdb_id = get_game_platform_igdb_id(&game, &db_conn).await?;
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
			let candidates = igdb_client
				.search_game_by_name_and_platform(&q, platform_igdb_id)
				.await?;

			let mut direct: Vec<ScoredCand<'_>> = vec![];
			let mut normalized: Vec<ScoredCand<'_>> = vec![];
			let mut needs_alt: Vec<(&IgdbGame, CandidateScore, Option<i16>)> = vec![];

			for c in &candidates {
				let candidate_year = c.first_release_date.and_then(year_from_unix_seconds);
				let candidate_platforms_i64 = c
					.platforms
					.as_ref()
					.map(|v| v.iter().map(|p| *p as i64).collect::<Vec<_>>());
				let parsed_cand = parse_name(&c.name);
				let gate = gate_and_score(
					&parsed_dat,
					Some(&parsed_cand),
					candidate_year,
					&[],
					&[],
					candidate_platforms_i64.as_deref(),
					Some(platform_igdb_id as i64),
					&[],
				);
				let score = match gate {
					CandidateGate::Reject => continue,
					CandidateGate::Pass(s) => s,
				};
				let year_i16 = candidate_year.and_then(|y| i16::try_from(y).ok());

				let cand_lower = c.name.to_lowercase();
				if cand_lower == q {
					direct.push(ScoredCand {
						cand: c,
						score,
						year: year_i16,
					});
					continue;
				}
				if normalize_title(&cand_lower) == q_norm {
					normalized.push(ScoredCand {
						cand: c,
						score,
						year: year_i16,
					});
					continue;
				}
				if c.alternative_names.is_some() {
					needs_alt.push((c, score, year_i16));
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

			let mut alt_direct: Vec<ScoredCand<'_>> = vec![];
			let mut alt_normalized: Vec<ScoredCand<'_>> = vec![];
			for (c, score, year_i16) in &needs_alt {
				let alt_ids = c.alternative_names.clone().expect("filtered above");
				let alt_resolved = igdb_client.get_alternative_names_by_id(alt_ids).await?;
				for alt in alt_resolved {
					let alt_lower = alt.name.to_lowercase();
					if alt_lower == q {
						alt_direct.push(ScoredCand {
							cand: c,
							score: *score,
							year: *year_i16,
						});
						break;
					}
					if normalize_title(&alt_lower) == q_norm {
						alt_normalized.push(ScoredCand {
							cand: c,
							score: *score,
							year: *year_i16,
						});
						break;
					}
				}
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
				"Cross-matched Game \"{}\" to IGDB Game ID {} via sibling \"{}\" ({label})",
				&game.name, s.cand.id, sibling
			);
			write_auto_match_success(
				"igdb",
				MetadataProviderEnum::Igdb,
				Target::Game(game.id),
				s.cand.id.to_string(),
				reason,
				Some(s.cand.name.clone()),
				s.year,
				db_conn,
				redis_conn,
			)
			.await?;
			Ok(true)
		}
		Selection::Ambiguous => {
			write_ambiguous(db_conn, redis_conn, game).await?;
			Ok(true)
		}
		Selection::None => Ok(false),
	}
}

fn year_from_unix_seconds(secs: i64) -> Option<u16> {
	DateTime::<Utc>::from_timestamp(secs, 0).and_then(|dt| u16::try_from(dt.year()).ok())
}
