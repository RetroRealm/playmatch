use crate::db::game::{
	find_game_parent, find_game_signature_metadata_mapping,
	get_automatic_match_failed_games_with_limit, get_unmatched_games_with_clone_of_with_limit,
	get_unmatched_games_without_clone_of_with_limit,
};
use crate::db::platform::{
	find_platform_of_game, find_platform_related_signature_metadata_mapping,
};
use crate::matching::util::{clean_name, normalize_title};
use crate::providers::igdb::IgdbClient;
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
	// Basic idea, first check if the parent game is matched to IGDB,
	// if yes, then we match to the same igdb id,
	// otherwise we try to match the game to igdb, if it succeeds, we apply the same igdb to the parent game

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

fn match_game_to_igdb(
	game: Model,
	igdb_client: Arc<IgdbClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		let mut redis_conn = igdb_client.redis_conn().clone();
		let platform_igdb_id = get_game_platform_igdb_id(&game, &db_conn).await?;

		let clean_name = clean_name(&game.name).to_lowercase();

		let search_results = igdb_client
			.search_game_by_name_and_platform(&clean_name, platform_igdb_id)
			.await?;

		for search_result in search_results {
			let search_result_name = search_result.name.to_lowercase();

			if search_result_name == clean_name {
				debug!(
					"Matched Game \"{}\" to IGDB Game ID {} (Direct Match)",
					&clean_name, search_result.id
				);
				write_auto_match_success(
					"igdb",
					MetadataProviderEnum::Igdb,
					Target::Game(game.id),
					search_result.id.to_string(),
					AutomaticMatchReasonEnum::DirectName,
					Some(search_result.name.clone()),
					&db_conn,
					&mut redis_conn,
				)
				.await?;

				return Ok(());
			}

			let search_result_name_normalized = normalize_title(&search_result_name);
			let clean_name_normalized = normalize_title(&clean_name);

			if search_result_name_normalized == clean_name_normalized {
				debug!(
					"Matched Game \"{}\" to IGDB Game ID {} (Normalized Name Match)",
					&clean_name, search_result.id
				);
				write_auto_match_success(
					"igdb",
					MetadataProviderEnum::Igdb,
					Target::Game(game.id),
					search_result.id.to_string(),
					AutomaticMatchReasonEnum::NormalizedName,
					Some(search_result.name.clone()),
					&db_conn,
					&mut redis_conn,
				)
				.await?;

				return Ok(());
			}

			if let Some(alternative_names) = search_result.alternative_names {
				debug!(
					"Game {} has no direct match but has alternative names, checking alternative names...",
					&clean_name
				);

				let alternative_names_resolved = igdb_client
					.get_alternative_names_by_id(alternative_names)
					.await?;

				for alternative_name in alternative_names_resolved {
					let alternative_name_lower = alternative_name.name.to_lowercase();

					if alternative_name_lower == clean_name {
						debug!(
							"Matched Game \"{}\" to IGDB Game ID {} (Alternative Name Match)",
							&clean_name, search_result.id
						);
						write_auto_match_success(
							"igdb",
							MetadataProviderEnum::Igdb,
							Target::Game(game.id),
							search_result.id.to_string(),
							AutomaticMatchReasonEnum::AlternativeName,
							Some(search_result.name.clone()),
							&db_conn,
							&mut redis_conn,
						)
						.await?;

						return Ok(());
					}

					let alternative_name_normalized = normalize_title(&alternative_name_lower);

					if alternative_name_normalized == clean_name_normalized {
						debug!(
							"Matched Game \"{}\" to IGDB Game ID {} (Normalized Alternative Name Match)",
							&clean_name, search_result.id
						);
						write_auto_match_success(
							"igdb",
							MetadataProviderEnum::Igdb,
							Target::Game(game.id),
							search_result.id.to_string(),
							AutomaticMatchReasonEnum::NormalizedAlternativeName,
							Some(search_result.name.clone()),
							&db_conn,
							&mut redis_conn,
						)
						.await?;

						return Ok(());
					}
				}
			}
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

	let platform_id_parsed = platform_igdb_metadata_mapping
		.provider_id
		.map(|id| id.parse::<i32>().unwrap());

	let platform_igdb_id = match platform_id_parsed {
		None => {
			return Err(anyhow::anyhow!(
				"Platform {} is missing its igdb id on its metadata mapping, this shouldn't happen...",
				&platform.name
			));
		}
		Some(platform_igdb_id) => platform_igdb_id,
	};

	Ok(platform_igdb_id)
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
		let cleaned_playmatch = clean_name(&game.name).to_lowercase();
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

			for c in &candidates {
				if c.name.to_lowercase() == q {
					debug!(
						"Cross-matched Game \"{}\" to IGDB Game ID {} via sibling \"{}\" (Direct)",
						&game.name, c.id, &sibling
					);
					write_auto_match_success(
						"igdb",
						MetadataProviderEnum::Igdb,
						Target::Game(game.id),
						c.id.to_string(),
						AutomaticMatchReasonEnum::CrossProviderDirectName,
						Some(c.name.clone()),
						&db_conn,
						&mut redis_conn,
					)
					.await?;
					return Ok(());
				}
			}
			for c in &candidates {
				if normalize_title(&c.name.to_lowercase()) == q_norm {
					debug!(
						"Cross-matched Game \"{}\" to IGDB Game ID {} via sibling \"{}\" (Normalized)",
						&game.name, c.id, &sibling
					);
					write_auto_match_success(
						"igdb",
						MetadataProviderEnum::Igdb,
						Target::Game(game.id),
						c.id.to_string(),
						AutomaticMatchReasonEnum::CrossProviderNormalizedName,
						Some(c.name.clone()),
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
