use crate::db::game::{
	find_game_parent, get_automatic_match_failed_games_with_limit,
	get_unmatched_games_with_clone_of_with_limit, get_unmatched_games_without_clone_of_with_limit,
};
use crate::db::signature_metadata_mapping::find_signature_metadata_mapping_by_platform_game_company_and_provider;
use crate::matching::util::{clean_name, normalize_title};
use crate::providers::steamgriddb::SteamGridDbClient;
use crate::providers::{
	DEFAULT_CHUNK_SIZE, Target, drive_match_pipeline, write_auto_match_failed,
	write_auto_match_success,
};
use entity::game::Model;
use entity::sea_orm_active_enums::{
	AutomaticMatchReasonEnum, FailedMatchReasonEnum, MatchTypeEnum, MetadataProviderEnum,
};
use futures_util::future::BoxFuture;
use log::debug;
use sea_orm::DbConn;
use std::sync::Arc;

pub async fn match_games_to_steamgriddb(
	client: Arc<SteamGridDbClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	drive_match_pipeline(
		"game",
		MetadataProviderEnum::Steamgriddb,
		get_unmatched_games_without_clone_of_with_limit,
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
		get_unmatched_games_with_clone_of_with_limit,
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
	Box::pin(async move {
		let parent_game = find_game_parent(&game, &db_conn).await?;

		if let Some(parent_game) = parent_game {
			let parent_mapping =
				find_signature_metadata_mapping_by_platform_game_company_and_provider(
					None,
					Some(parent_game.id),
					None,
					MetadataProviderEnum::Steamgriddb,
					&db_conn,
				)
				.await?;

			if let Some(mapping) = &parent_mapping
				&& matches!(
					mapping.match_type,
					MatchTypeEnum::Automatic | MatchTypeEnum::Manual
				) && let Some(provider_id) = mapping.provider_id.clone()
			{
				debug!(
					"Matched Game \"{}\" to SteamGridDB Game ID {provider_id} (Via Parent)",
					&game.name
				);
				write_auto_match_success(
					"steamgriddb",
					MetadataProviderEnum::Steamgriddb,
					Target::Game(game.id),
					provider_id,
					AutomaticMatchReasonEnum::ViaParent,
					&db_conn,
				)
				.await?;
				return Ok(());
			}

			match_game_to_steamgriddb(game.clone(), client.clone(), db_conn.clone()).await?;

			let mapping = find_signature_metadata_mapping_by_platform_game_company_and_provider(
				None,
				Some(game.id),
				None,
				MetadataProviderEnum::Steamgriddb,
				&db_conn,
			)
			.await?;

			if let Some(mapping) = mapping
				&& matches!(
					mapping.match_type,
					MatchTypeEnum::Automatic | MatchTypeEnum::Manual
				) && let Some(provider_id) = mapping.provider_id
			{
				debug!("Propagating SteamGridDB match from clone to parent game (Via Child)");
				write_auto_match_success(
					"steamgriddb",
					MetadataProviderEnum::Steamgriddb,
					Target::Game(parent_game.id),
					provider_id,
					AutomaticMatchReasonEnum::ViaChild,
					&db_conn,
				)
				.await?;
			}
		}

		Ok(())
	})
}

fn match_game_to_steamgriddb(
	game: Model,
	client: Arc<SteamGridDbClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		let cleaned = clean_name(&game.name).to_lowercase();
		let cleaned_normalized = normalize_title(&cleaned);

		let candidates = client.search_games(&cleaned).await?;

		for candidate in &candidates {
			let candidate_lower = candidate.name.to_lowercase();
			if candidate_lower == cleaned {
				debug!(
					"Matched Game \"{}\" to SteamGridDB Game ID {} (Direct Match)",
					&cleaned, candidate.id
				);
				write_auto_match_success(
					"steamgriddb",
					MetadataProviderEnum::Steamgriddb,
					Target::Game(game.id),
					candidate.id.to_string(),
					AutomaticMatchReasonEnum::DirectName,
					&db_conn,
				)
				.await?;
				return Ok(());
			}
		}

		for candidate in &candidates {
			let candidate_lower = candidate.name.to_lowercase();
			if normalize_title(&candidate_lower) == cleaned_normalized {
				debug!(
					"Matched Game \"{}\" to SteamGridDB Game ID {} (Normalized Name Match)",
					&cleaned, candidate.id
				);
				write_auto_match_success(
					"steamgriddb",
					MetadataProviderEnum::Steamgriddb,
					Target::Game(game.id),
					candidate.id.to_string(),
					AutomaticMatchReasonEnum::NormalizedName,
					&db_conn,
				)
				.await?;
				return Ok(());
			}
		}

		debug!("No SteamGridDB match found for Game \"{}\"", &cleaned);
		write_auto_match_failed(
			"steamgriddb",
			MetadataProviderEnum::Steamgriddb,
			Target::Game(game.id),
			FailedMatchReasonEnum::NoDirectMatch,
			&db_conn,
		)
		.await?;

		Ok(())
	})
}
