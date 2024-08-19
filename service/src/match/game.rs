use crate::db::game::get_unmatched_games_without_clone_of_id;
use crate::db::platform::{find_platform_of_game, find_related_signature_metadata_mapping};
use crate::db::signature_metadata_mapping::{
	create_or_update_signature_metadata_mapping, SignatureMetadataMappingInput,
};
use crate::metadata::igdb::IgdbClient;
use crate::r#match::{clean_name, PAGE_SIZE};
use entity::sea_orm_active_enums::{
	AutomaticMatchReasonEnum, FailedMatchReasonEnum, MatchTypeEnum, MetadataProviderEnum,
};
use log::debug;
use sea_orm::DbConn;
use std::sync::Arc;

pub async fn match_games_to_igdb(
	igdb_client: Arc<IgdbClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	let mut game_paginator = get_unmatched_games_without_clone_of_id(PAGE_SIZE, db_conn);

	while let Some(games) = game_paginator.fetch_and_next().await? {
		for game_chunk in games.chunks(4) {
			let mut results = vec![];

			for game in game_chunk.iter().cloned() {
				let igdb_client = igdb_client.clone();
				let db_conn = db_conn.clone();
				results.push(tokio::spawn(async move {
					match_game_to_igdb(game, &db_conn, igdb_client.clone()).await
				}))
			}

			for result in results {
				result.await??;
			}
		}
	}

	Ok(())
}

pub async fn match_game_to_igdb(
	game: entity::game::Model,
	db_conn: &DbConn,
	igdb_client: Arc<IgdbClient>,
) -> anyhow::Result<()> {
	let platform_igdb_id = get_game_platform_igdb_id(&game, db_conn).await?;

	let clean_name = clean_name(&game.name);

	debug!("Cleaning Game name \"{}\" to \"{}\"", game.name, clean_name);

	let search_results = igdb_client
		.search_game_by_name_and_platform(&clean_name, platform_igdb_id)
		.await
		.map_err(|e| {
			debug!(
				"Failed to search for Game \"{}\" on IGDB: {}",
				&clean_name, e
			);
			e
		})?;

	let mut matched = false;

	for search_result in search_results {
		if search_result.name.to_lowercase() == clean_name.to_lowercase() {
			debug!(
				"Matched Game \"{}\" to IGDB Game ID {} (Direct Match)",
				&clean_name, search_result.id
			);
			matched = true;
			create_or_update_signature_metadata_mapping(
				SignatureMetadataMappingInput {
					provider_name: MetadataProviderEnum::Igdb,
					provider_id: Some(search_result.id.to_string()),
					comment: None,
					company_id: None,
					game_id: Some(game.id),
					platform_id: None,
					match_type: MatchTypeEnum::Automatic,
					manual_match_type: None,
					failed_match_reason: None,
					automatic_match_reason: Some(AutomaticMatchReasonEnum::DirectName),
				},
				db_conn,
			)
			.await?;

			break;
		}

		debug!(
			"Game {} has no direct match, checking alternative names...",
			&clean_name
		);

		if let Some(alternative_names) = search_result.alternative_names {
			let alternative_names_resolved = igdb_client
				.get_alternative_names_by_id(alternative_names)
				.await?;

			for alternative_name in alternative_names_resolved {
				if alternative_name.name.to_lowercase() == clean_name.to_lowercase() {
					debug!(
						"Matched Game \"{}\" to IGDB Game ID {} (Alternative Name Match)",
						&clean_name, search_result.id
					);
					matched = true;
					create_or_update_signature_metadata_mapping(
						SignatureMetadataMappingInput {
							provider_name: MetadataProviderEnum::Igdb,
							provider_id: Some(search_result.id.to_string()),
							comment: None,
							company_id: None,
							game_id: Some(game.id),
							platform_id: None,
							match_type: MatchTypeEnum::Automatic,
							manual_match_type: None,
							failed_match_reason: None,
							automatic_match_reason: Some(AutomaticMatchReasonEnum::AlternativeName),
						},
						db_conn,
					)
					.await?;

					break;
				}
			}
		}
	}

	if !matched {
		debug!("No match found for Game \"{}\"", &clean_name);
		create_or_update_signature_metadata_mapping(
			SignatureMetadataMappingInput {
				provider_name: MetadataProviderEnum::Igdb,
				provider_id: None,
				comment: None,
				company_id: None,
				game_id: Some(game.id),
				platform_id: None,
				match_type: MatchTypeEnum::Failed,
				manual_match_type: None,
				failed_match_reason: Some(FailedMatchReasonEnum::NoDirectMatch),
				automatic_match_reason: None,
			},
			db_conn,
		)
		.await?;
	}

	Ok(())
}

async fn get_game_platform_igdb_id(
	game: &entity::game::Model,
	db_conn: &DbConn,
) -> anyhow::Result<i32> {
	let platform = match find_platform_of_game(game.id, db_conn).await? {
		None => {
			return Err(anyhow::anyhow!(
				"No platform found for Game \"{}\", this shouldn't happen...",
				game.name
			));
		}
		Some(p) => p,
	};

	let platform_igdb_metadata_mapping =
		match find_related_signature_metadata_mapping(&platform, db_conn).await? {
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
