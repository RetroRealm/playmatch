use crate::db::game::{
	find_game_parent, find_game_signature_metadata_mapping,
	get_automatic_match_failed_games_with_limit, get_unmatched_games_with_clone_of_with_limit,
	get_unmatched_games_without_clone_of_with_limit,
};
use crate::db::game_file::get_game_files_from_game_id;
use crate::db::platform::{
	find_platform_of_game, find_platform_related_signature_metadata_mapping,
};
use crate::matching::util::{clean_name, normalize_title};
use crate::providers::screenscraper::ScreenScraperClient;
use crate::providers::screenscraper::model::SsGame;
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

pub async fn match_games_to_screenscraper(
	client: Arc<ScreenScraperClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	drive_match_pipeline(
		"game",
		MetadataProviderEnum::Screenscraper,
		get_unmatched_games_without_clone_of_with_limit,
		match_game_to_screenscraper,
		client.clone(),
		db_conn,
		DEFAULT_CHUNK_SIZE,
	)
	.await?;
	debug!("Finished matching games without clone_of id to ScreenScraper");

	drive_match_pipeline(
		"game",
		MetadataProviderEnum::Screenscraper,
		get_unmatched_games_with_clone_of_with_limit,
		match_clone_of_game_to_screenscraper,
		client.clone(),
		db_conn,
		DEFAULT_CHUNK_SIZE,
	)
	.await?;
	debug!("Finished matching games with clone_of id to ScreenScraper");

	drive_match_pipeline(
		"game",
		MetadataProviderEnum::Screenscraper,
		get_automatic_match_failed_games_with_limit,
		match_game_to_screenscraper,
		client,
		db_conn,
		DEFAULT_CHUNK_SIZE,
	)
	.await?;
	debug!("Finished retrying previously failed ScreenScraper game matches");

	Ok(())
}

fn match_clone_of_game_to_screenscraper(
	game: Model,
	client: Arc<ScreenScraperClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		if client.is_quota_exhausted() {
			return Ok(());
		}

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
					"Matched Game \"{}\" to ScreenScraper Game ID {provider_id} (Via Parent)",
					&game.name
				);
				write_auto_match_success(
					"screenscraper",
					MetadataProviderEnum::Screenscraper,
					Target::Game(game.id),
					provider_id,
					AutomaticMatchReasonEnum::ViaParent,
					&db_conn,
					&mut redis_conn,
				)
				.await?;
				return Ok(());
			}

			match_game_to_screenscraper(game.clone(), client.clone(), db_conn.clone()).await?;

			let mapping = find_game_signature_metadata_mapping(&game, &db_conn).await?;

			if let Some(mapping) = mapping
				&& matches!(
					mapping.match_type,
					MatchTypeEnum::Automatic | MatchTypeEnum::Manual
				) && let Some(provider_id) = mapping.provider_id
			{
				debug!("Propagating ScreenScraper match from clone to parent game (Via Child)");
				write_auto_match_success(
					"screenscraper",
					MetadataProviderEnum::Screenscraper,
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

fn match_game_to_screenscraper(
	game: Model,
	client: Arc<ScreenScraperClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		if client.is_quota_exhausted() {
			return Ok(());
		}

		let mut redis_conn = client.redis_conn().clone();
		let system_id = get_game_platform_screenscraper_id(&game, &db_conn).await?;

		if let Some(()) =
			try_match_by_hashes(&game, system_id, &client, &db_conn, &mut redis_conn).await?
		{
			return Ok(());
		}

		if client.is_quota_exhausted() {
			return Ok(());
		}

		let cleaned = clean_name(&game.name).to_lowercase();
		let cleaned_normalized = normalize_title(&cleaned);

		let candidates = client.search_games(system_id, &cleaned).await?;

		for candidate in &candidates {
			for name in candidate.iter_candidate_names() {
				if name.to_lowercase() == cleaned {
					debug!(
						"Matched Game \"{}\" to ScreenScraper Game ID {} (Direct Match)",
						&cleaned, candidate.id
					);
					write_auto_match_success(
						"screenscraper",
						MetadataProviderEnum::Screenscraper,
						Target::Game(game.id),
						candidate.id.to_string(),
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
			for name in candidate.iter_candidate_names() {
				if normalize_title(&name.to_lowercase()) == cleaned_normalized {
					debug!(
						"Matched Game \"{}\" to ScreenScraper Game ID {} (Normalized Match)",
						&cleaned, candidate.id
					);
					write_auto_match_success(
						"screenscraper",
						MetadataProviderEnum::Screenscraper,
						Target::Game(game.id),
						candidate.id.to_string(),
						AutomaticMatchReasonEnum::NormalizedName,
						&db_conn,
						&mut redis_conn,
					)
					.await?;
					return Ok(());
				}
			}
		}

		debug!("No ScreenScraper match found for Game \"{}\"", &cleaned);
		write_auto_match_failed(
			"screenscraper",
			MetadataProviderEnum::Screenscraper,
			Target::Game(game.id),
			FailedMatchReasonEnum::NoDirectMatch,
			&db_conn,
			&mut redis_conn,
		)
		.await?;

		Ok(())
	})
}

/// Walk every game file's md5/sha1/crc against `jeuInfos.php` until a hit
/// lands or all hashes are exhausted. Returns `Some(())` when a match was
/// recorded so the caller skips the name ladder.
async fn try_match_by_hashes(
	game: &Model,
	system_id: i32,
	client: &ScreenScraperClient,
	db_conn: &DbConn,
	redis_conn: &mut redis::aio::MultiplexedConnection,
) -> anyhow::Result<Option<()>> {
	let files = get_game_files_from_game_id(game.id, db_conn).await?;
	if files.is_empty() {
		return Ok(None);
	}

	for file in &files {
		if client.is_quota_exhausted() {
			return Ok(None);
		}
		if let Some(md5) = file.md5.as_deref().filter(|s| !s.is_empty())
			&& let Some(found) = client.get_game_by_md5(system_id, md5).await?
		{
			record_hash_match(
				game,
				&found,
				AutomaticMatchReasonEnum::Md5Hash,
				db_conn,
				redis_conn,
			)
			.await?;
			return Ok(Some(()));
		}
		if client.is_quota_exhausted() {
			return Ok(None);
		}
		if let Some(sha1) = file.sha1.as_deref().filter(|s| !s.is_empty())
			&& let Some(found) = client.get_game_by_sha1(system_id, sha1).await?
		{
			record_hash_match(
				game,
				&found,
				AutomaticMatchReasonEnum::Sha1Hash,
				db_conn,
				redis_conn,
			)
			.await?;
			return Ok(Some(()));
		}
		if client.is_quota_exhausted() {
			return Ok(None);
		}
		if let Some(crc) = file.crc.as_deref().filter(|s| !s.is_empty())
			&& let Some(found) = client.get_game_by_crc(system_id, crc).await?
		{
			record_hash_match(
				game,
				&found,
				AutomaticMatchReasonEnum::CrcHash,
				db_conn,
				redis_conn,
			)
			.await?;
			return Ok(Some(()));
		}
	}

	Ok(None)
}

async fn record_hash_match(
	game: &Model,
	found: &SsGame,
	reason: AutomaticMatchReasonEnum,
	db_conn: &DbConn,
	redis_conn: &mut redis::aio::MultiplexedConnection,
) -> anyhow::Result<()> {
	debug!(
		"Matched Game \"{}\" to ScreenScraper Game ID {} ({:?})",
		game.name, found.id, reason
	);
	write_auto_match_success(
		"screenscraper",
		MetadataProviderEnum::Screenscraper,
		Target::Game(game.id),
		found.id.to_string(),
		reason,
		db_conn,
		redis_conn,
	)
	.await
}

async fn get_game_platform_screenscraper_id(game: &Model, db_conn: &DbConn) -> anyhow::Result<i32> {
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
		MetadataProviderEnum::Screenscraper,
		db_conn,
	)
	.await?
	.ok_or_else(|| {
		anyhow::anyhow!(
			"Platform {} is missing its screenscraper metadata mapping, this shouldn't happen...",
			&platform.name
		)
	})?;

	if !matches!(
		mapping.match_type,
		MatchTypeEnum::Automatic | MatchTypeEnum::Manual
	) {
		return Err(anyhow::anyhow!(
			"Platform {} is not matched to ScreenScraper, this shouldn't happen...",
			&platform.name
		));
	}

	let provider_id = mapping.provider_id.ok_or_else(|| {
		anyhow::anyhow!(
			"Platform {} screenscraper mapping has no provider_id",
			&platform.name
		)
	})?;
	provider_id.parse::<i32>().map_err(|e| {
		anyhow::anyhow!(
			"Platform {} screenscraper provider_id {provider_id:?} is not a valid i32: {e}",
			&platform.name
		)
	})
}
