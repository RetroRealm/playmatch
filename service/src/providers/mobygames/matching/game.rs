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
use crate::providers::mobygames::MobyGamesClient;
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
					&db_conn,
					&mut redis_conn,
				)
				.await?;
			}
		}

		Ok(())
	})
}

fn match_game_to_mobygames(
	game: Model,
	client: Arc<MobyGamesClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		let mut redis_conn = client.redis_conn().clone();
		let platform_id = get_game_platform_mobygames_id(&game, &db_conn).await?;

		let cleaned = clean_name(&game.name).to_lowercase();
		let cleaned_normalized = normalize_title(&cleaned);

		let candidates = client.search_games(Some(platform_id), &cleaned).await?;

		for candidate in &candidates {
			for name in candidate.iter_candidate_titles() {
				if name.to_lowercase() == cleaned {
					debug!(
						"Matched Game \"{}\" to MobyGames Game ID {} (Direct Match)",
						&cleaned, candidate.game_id
					);
					write_auto_match_success(
						"mobygames",
						MetadataProviderEnum::Mobygames,
						Target::Game(game.id),
						candidate.game_id.to_string(),
						AutomaticMatchReasonEnum::DirectName,
						&db_conn,
						&mut redis_conn,
					)
					.await?;
					return Ok(());
				}
			}
		}

		for candidate in &candidates {
			for name in candidate.iter_candidate_titles() {
				if normalize_title(&name.to_lowercase()) == cleaned_normalized {
					debug!(
						"Matched Game \"{}\" to MobyGames Game ID {} (Normalized Match)",
						&cleaned, candidate.game_id
					);
					write_auto_match_success(
						"mobygames",
						MetadataProviderEnum::Mobygames,
						Target::Game(game.id),
						candidate.game_id.to_string(),
						AutomaticMatchReasonEnum::NormalizedName,
						&db_conn,
						&mut redis_conn,
					)
					.await?;
					return Ok(());
				}
			}
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
