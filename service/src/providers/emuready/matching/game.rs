use crate::db::game::{
	find_game_parent, find_game_signature_metadata_mapping,
	get_automatic_match_failed_games_with_limit, get_unmatched_games_with_clone_of_with_limit,
	get_unmatched_games_without_clone_of_with_limit,
};
use crate::db::platform::{
	find_platform_of_game, find_platform_related_signature_metadata_mapping,
};
use crate::matching::util::{clean_name, normalize_title};
use crate::providers::MetadataProvider;
use crate::providers::emuready::EmuReadyClient;
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

pub async fn match_games_to_emuready(
	client: Arc<EmuReadyClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	let chunk_size = client.chunk_size();
	drive_match_pipeline(
		"game",
		MetadataProviderEnum::EmuReady,
		get_unmatched_games_without_clone_of_with_limit,
		match_game_to_emuready,
		client.clone(),
		db_conn,
		chunk_size,
	)
	.await?;
	debug!("Finished matching games without clone_of id to EmuReady");

	drive_match_pipeline(
		"game",
		MetadataProviderEnum::EmuReady,
		get_unmatched_games_with_clone_of_with_limit,
		match_clone_of_game_to_emuready,
		client.clone(),
		db_conn,
		chunk_size,
	)
	.await?;
	debug!("Finished matching games with clone_of id to EmuReady");

	drive_match_pipeline(
		"game",
		MetadataProviderEnum::EmuReady,
		get_automatic_match_failed_games_with_limit,
		match_game_to_emuready,
		client,
		db_conn,
		chunk_size,
	)
	.await?;
	debug!("Finished retrying previously failed EmuReady game matches");

	Ok(())
}

fn match_clone_of_game_to_emuready(
	game: Model,
	client: Arc<EmuReadyClient>,
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
					"Matched Game \"{}\" to EmuReady Game ID {provider_id} (Via Parent)",
					&game.name
				);
				write_auto_match_success(
					"emuready",
					MetadataProviderEnum::EmuReady,
					Target::Game(game.id),
					provider_id,
					AutomaticMatchReasonEnum::ViaParent,
					mapping.matched_name.clone(),
					&db_conn,
					&mut redis_conn,
				)
				.await?;
				return Ok(());
			}

			match_game_to_emuready(game.clone(), client.clone(), db_conn.clone()).await?;

			let mapping = find_game_signature_metadata_mapping(&game, &db_conn).await?;

			if let Some(mapping) = mapping
				&& matches!(
					mapping.match_type,
					MatchTypeEnum::Automatic | MatchTypeEnum::Manual
				) && let Some(provider_id) = mapping.provider_id
			{
				debug!("Propagating EmuReady match from clone to parent game (Via Child)");
				write_auto_match_success(
					"emuready",
					MetadataProviderEnum::EmuReady,
					Target::Game(parent_game.id),
					provider_id,
					AutomaticMatchReasonEnum::ViaChild,
					mapping.matched_name,
					&db_conn,
					&mut redis_conn,
				)
				.await?;
			}
		}

		Ok(())
	})
}

fn match_game_to_emuready(
	game: Model,
	client: Arc<EmuReadyClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		let mut redis_conn = client.redis_conn().clone();
		let system_id = get_game_platform_emuready_id(&game, &db_conn).await?;

		let cleaned = clean_name(&game.name).to_lowercase();
		let cleaned_normalized = normalize_title(&cleaned);

		let candidates = client.search_games(&system_id, &cleaned).await?;

		for candidate in &candidates {
			if candidate.title.to_lowercase() == cleaned {
				debug!(
					"Matched Game \"{}\" to EmuReady Game ID {} (Direct Match)",
					&cleaned, candidate.id
				);
				write_auto_match_success(
					"emuready",
					MetadataProviderEnum::EmuReady,
					Target::Game(game.id),
					candidate.id.clone(),
					AutomaticMatchReasonEnum::DirectName,
					Some(candidate.title.clone()),
					&db_conn,
					&mut redis_conn,
				)
				.await?;
				return Ok(());
			}
		}

		for candidate in &candidates {
			let normalised = candidate
				.normalized_title
				.clone()
				.unwrap_or_else(|| normalize_title(&candidate.title.to_lowercase()));
			if normalised == cleaned_normalized {
				debug!(
					"Matched Game \"{}\" to EmuReady Game ID {} (Normalized Match)",
					&cleaned, candidate.id
				);
				write_auto_match_success(
					"emuready",
					MetadataProviderEnum::EmuReady,
					Target::Game(game.id),
					candidate.id.clone(),
					AutomaticMatchReasonEnum::NormalizedName,
					Some(candidate.title.clone()),
					&db_conn,
					&mut redis_conn,
				)
				.await?;
				return Ok(());
			}
		}

		debug!("No EmuReady match found for Game \"{}\"", &cleaned);
		write_auto_match_failed(
			"emuready",
			MetadataProviderEnum::EmuReady,
			Target::Game(game.id),
			FailedMatchReasonEnum::NoDirectMatch,
			&db_conn,
			&mut redis_conn,
		)
		.await?;

		Ok(())
	})
}

async fn get_game_platform_emuready_id(game: &Model, db_conn: &DbConn) -> anyhow::Result<String> {
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
		MetadataProviderEnum::EmuReady,
		db_conn,
	)
	.await?
	.ok_or_else(|| {
		anyhow::anyhow!(
			"Platform {} is missing its emuready metadata mapping, this shouldn't happen...",
			&platform.name
		)
	})?;

	if !matches!(
		mapping.match_type,
		MatchTypeEnum::Automatic | MatchTypeEnum::Manual
	) {
		return Err(anyhow::anyhow!(
			"Platform {} is not matched to EmuReady, this shouldn't happen...",
			&platform.name
		));
	}

	mapping.provider_id.ok_or_else(|| {
		anyhow::anyhow!(
			"Platform {} emuready mapping has no provider_id",
			&platform.name
		)
	})
}

pub fn match_game_via_sibling_name_emuready(
	game: Model,
	sibling_names: Vec<String>,
	client: Arc<EmuReadyClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		let mut redis_conn = client.redis_conn().clone();
		let system_id = get_game_platform_emuready_id(&game, &db_conn).await?;
		let cleaned_playmatch = clean_name(&game.name).to_lowercase();
		let mut tried: std::collections::HashSet<String> = std::collections::HashSet::new();
		tried.insert(cleaned_playmatch);

		for sibling in sibling_names {
			let q = clean_name(&sibling).to_lowercase();
			if !tried.insert(q.clone()) {
				continue;
			}
			let q_norm = normalize_title(&q);
			let candidates = client.search_games(&system_id, &q).await?;

			for c in &candidates {
				if c.title.to_lowercase() == q {
					debug!(
						"Cross-matched Game \"{}\" to EmuReady Game ID {} via sibling \"{}\" (Direct)",
						&game.name, c.id, &sibling
					);
					write_auto_match_success(
						"emuready",
						MetadataProviderEnum::EmuReady,
						Target::Game(game.id),
						c.id.clone(),
						AutomaticMatchReasonEnum::CrossProviderDirectName,
						Some(c.title.clone()),
						&db_conn,
						&mut redis_conn,
					)
					.await?;
					return Ok(());
				}
			}
			for c in &candidates {
				let normalised = c
					.normalized_title
					.clone()
					.unwrap_or_else(|| normalize_title(&c.title.to_lowercase()));
				if normalised == q_norm {
					debug!(
						"Cross-matched Game \"{}\" to EmuReady Game ID {} via sibling \"{}\" (Normalized)",
						&game.name, c.id, &sibling
					);
					write_auto_match_success(
						"emuready",
						MetadataProviderEnum::EmuReady,
						Target::Game(game.id),
						c.id.clone(),
						AutomaticMatchReasonEnum::CrossProviderNormalizedName,
						Some(c.title.clone()),
						&db_conn,
						&mut redis_conn,
					)
					.await?;
					return Ok(());
				}
			}
		}
		Ok(())
	})
}
