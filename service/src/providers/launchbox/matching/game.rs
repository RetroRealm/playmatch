use crate::db::game::{
	get_automatic_match_failed_games_with_limit, get_unmatched_games_with_clone_of_with_limit,
	get_unmatched_games_without_clone_of_with_limit,
};
use crate::db::launchbox::{
	find_lb_game_by_platform_and_alternate_name_region_priority,
	find_lb_game_by_platform_and_alternate_normalized_name_region_priority,
	find_lb_game_by_platform_and_name, find_lb_game_by_platform_and_normalized_name,
};
use crate::db::platform::{
	find_platform_of_game, find_platform_related_signature_metadata_mapping,
};
use crate::matching::name_parse::{ParsedName, parse_name};
use crate::matching::scoring::{CandidateGate, gate_and_score};
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

fn lb_passes_gate(parsed_dat: &ParsedName, found: &launchbox_game::Model) -> bool {
	let parsed_cand = parse_name(&found.name);
	let candidate_year = found.release_year.and_then(|y| u16::try_from(y).ok());
	matches!(
		gate_and_score(
			parsed_dat,
			Some(&parsed_cand),
			candidate_year,
			&[],
			&[],
			None,
			None,
			&[]
		),
		CandidateGate::Pass(_)
	)
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

		if let Some(found) =
			find_lb_game_by_platform_and_name(&platform_name, &cleaned, &db_conn).await?
			&& lb_passes_gate(&parsed_dat, &found)
		{
			debug!(
				"Matched Game \"{}\" to LaunchBox Game ID {} (Direct Match)",
				&cleaned, found.database_id
			);
			write_auto_match_success(
				"launchbox",
				MetadataProviderEnum::Launchbox,
				Target::Game(game.id),
				found.database_id.to_string(),
				AutomaticMatchReasonEnum::DirectName,
				Some(found.name.clone()),
				lb_year(&found),
				&db_conn,
				&mut redis_conn,
			)
			.await?;
			return Ok(());
		}

		if let Some(found) = find_lb_game_by_platform_and_alternate_name_region_priority(
			&platform_name,
			&cleaned,
			&dat_lb_regions,
			&db_conn,
		)
		.await? && lb_passes_gate(&parsed_dat, &found)
		{
			debug!(
				"Matched Game \"{}\" to LaunchBox Game ID {} (Alternative Name)",
				&cleaned, found.database_id
			);
			write_auto_match_success(
				"launchbox",
				MetadataProviderEnum::Launchbox,
				Target::Game(game.id),
				found.database_id.to_string(),
				AutomaticMatchReasonEnum::AlternativeName,
				Some(found.name.clone()),
				lb_year(&found),
				&db_conn,
				&mut redis_conn,
			)
			.await?;
			return Ok(());
		}

		if let Some(found) = find_lb_game_by_platform_and_normalized_name(
			&platform_name,
			&cleaned_normalized,
			&db_conn,
		)
		.await? && lb_passes_gate(&parsed_dat, &found)
		{
			debug!(
				"Matched Game \"{}\" to LaunchBox Game ID {} (Normalized Match)",
				&cleaned, found.database_id
			);
			write_auto_match_success(
				"launchbox",
				MetadataProviderEnum::Launchbox,
				Target::Game(game.id),
				found.database_id.to_string(),
				AutomaticMatchReasonEnum::NormalizedName,
				Some(found.name.clone()),
				lb_year(&found),
				&db_conn,
				&mut redis_conn,
			)
			.await?;
			return Ok(());
		}

		if let Some(found) = find_lb_game_by_platform_and_alternate_normalized_name_region_priority(
			&platform_name,
			&cleaned_normalized,
			&dat_lb_regions,
			&db_conn,
		)
		.await? && lb_passes_gate(&parsed_dat, &found)
		{
			debug!(
				"Matched Game \"{}\" to LaunchBox Game ID {} (Normalized Alternative Name)",
				&cleaned, found.database_id
			);
			write_auto_match_success(
				"launchbox",
				MetadataProviderEnum::Launchbox,
				Target::Game(game.id),
				found.database_id.to_string(),
				AutomaticMatchReasonEnum::NormalizedAlternativeName,
				Some(found.name.clone()),
				lb_year(&found),
				&db_conn,
				&mut redis_conn,
			)
			.await?;
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
		let dat_lb_regions: Vec<&'static str> = parsed_dat
			.regions
			.iter()
			.flat_map(|r| r.lb_codes().iter().copied())
			.collect();
		let mut tried: std::collections::HashSet<String> = std::collections::HashSet::new();
		tried.insert(cleaned_playmatch);

		for sibling in sibling_names {
			let q = clean_name(&sibling).to_lowercase();
			if !tried.insert(q.clone()) {
				continue;
			}
			let q_norm = normalize_title(&q);

			if let Some(found) =
				find_lb_game_by_platform_and_name(&platform_name, &q, &db_conn).await?
				&& lb_passes_gate(&parsed_dat, &found)
			{
				debug!(
					"Cross-matched Game \"{}\" to LaunchBox Game ID {} via sibling \"{}\" (Direct)",
					&game.name, found.database_id, &sibling
				);
				write_auto_match_success(
					"launchbox",
					MetadataProviderEnum::Launchbox,
					Target::Game(game.id),
					found.database_id.to_string(),
					AutomaticMatchReasonEnum::CrossProviderDirectName,
					Some(found.name.clone()),
					lb_year(&found),
					&db_conn,
					&mut redis_conn,
				)
				.await?;
				return Ok(());
			}

			if let Some(found) = find_lb_game_by_platform_and_alternate_name_region_priority(
				&platform_name,
				&q,
				&dat_lb_regions,
				&db_conn,
			)
			.await? && lb_passes_gate(&parsed_dat, &found)
			{
				debug!(
					"Cross-matched Game \"{}\" to LaunchBox Game ID {} via sibling \"{}\" (Direct alt)",
					&game.name, found.database_id, &sibling
				);
				write_auto_match_success(
					"launchbox",
					MetadataProviderEnum::Launchbox,
					Target::Game(game.id),
					found.database_id.to_string(),
					AutomaticMatchReasonEnum::CrossProviderDirectName,
					Some(found.name.clone()),
					lb_year(&found),
					&db_conn,
					&mut redis_conn,
				)
				.await?;
				return Ok(());
			}

			if let Some(found) =
				find_lb_game_by_platform_and_normalized_name(&platform_name, &q_norm, &db_conn)
					.await? && lb_passes_gate(&parsed_dat, &found)
			{
				debug!(
					"Cross-matched Game \"{}\" to LaunchBox Game ID {} via sibling \"{}\" (Normalized)",
					&game.name, found.database_id, &sibling
				);
				write_auto_match_success(
					"launchbox",
					MetadataProviderEnum::Launchbox,
					Target::Game(game.id),
					found.database_id.to_string(),
					AutomaticMatchReasonEnum::CrossProviderNormalizedName,
					Some(found.name.clone()),
					lb_year(&found),
					&db_conn,
					&mut redis_conn,
				)
				.await?;
				return Ok(());
			}

			if let Some(found) =
				find_lb_game_by_platform_and_alternate_normalized_name_region_priority(
					&platform_name,
					&q_norm,
					&dat_lb_regions,
					&db_conn,
				)
				.await? && lb_passes_gate(&parsed_dat, &found)
			{
				debug!(
					"Cross-matched Game \"{}\" to LaunchBox Game ID {} via sibling \"{}\" (Normalized alt)",
					&game.name, found.database_id, &sibling
				);
				write_auto_match_success(
					"launchbox",
					MetadataProviderEnum::Launchbox,
					Target::Game(game.id),
					found.database_id.to_string(),
					AutomaticMatchReasonEnum::CrossProviderNormalizedName,
					Some(found.name.clone()),
					lb_year(&found),
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
