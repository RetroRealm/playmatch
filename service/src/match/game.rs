use crate::db::game::get_unmatched_games_without_clone_of_id;
use crate::db::platform::{find_platform_of_game, find_related_signature_metadata_mapping};
use crate::db::signature_metadata_mapping::{
	create_or_update_signature_metadata_mapping, SignatureMetadataMappingInputBuilder,
};
use crate::metadata::igdb::IgdbClient;
use crate::r#match::{clean_name, handle_db_pagination_chunked, PAGE_SIZE};
use entity::sea_orm_active_enums::{
	AutomaticMatchReasonEnum, FailedMatchReasonEnum, MatchTypeEnum, MetadataProviderEnum,
};
use log::{debug, error, info};
use sea_orm::prelude::Uuid;
use sea_orm::DbConn;
use std::sync::Arc;

pub async fn match_games_to_igdb(
	igdb_client: Arc<IgdbClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	let game_paginator = get_unmatched_games_without_clone_of_id(PAGE_SIZE, db_conn);

	handle_db_pagination_chunked(
		game_paginator,
		igdb_client,
		db_conn.clone(),
		|t, arc, connection| {
			tokio::spawn(async move { match_game_to_igdb(t, arc, connection).await })
		},
	)
	.await?;
	info!("Finished matching games without clone_of id to IGDB");

	Ok(())
}

async fn match_game_to_igdb(
	game: entity::game::Model,
	igdb_client: Arc<IgdbClient>,
	db_conn: DbConn,
) -> anyhow::Result<()> {
	let platform_igdb_id = get_game_platform_igdb_id(&game, &db_conn).await?;

	let clean_name = clean_name(&game.name);

	let search_results = igdb_client
		.search_game_by_name_and_platform(&clean_name, platform_igdb_id)
		.await
		.map_err(|e| {
			error!(
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
			create_or_update_signature_metadata_mapping_success(
				search_result.id.to_string(),
				game.id,
				AutomaticMatchReasonEnum::DirectName,
				&db_conn,
			)
			.await?;

			break;
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
				if alternative_name.name.to_lowercase() == clean_name.to_lowercase() {
					debug!(
						"Matched Game \"{}\" to IGDB Game ID {} (Alternative Name Match)",
						&clean_name, search_result.id
					);
					matched = true;
					create_or_update_signature_metadata_mapping_success(
						search_result.id.to_string(),
						game.id,
						AutomaticMatchReasonEnum::AlternativeName,
						&db_conn,
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
			SignatureMetadataMappingInputBuilder::default()
				.provider(MetadataProviderEnum::Igdb)
				.game_id(Some(game.id))
				.match_type(MatchTypeEnum::Failed)
				.failed_match_reason(Some(FailedMatchReasonEnum::NoDirectMatch))
				.build()?,
			&db_conn,
		)
		.await?;
	}

	Ok(())
}

async fn create_or_update_signature_metadata_mapping_success(
	provider_id: String,
	game_id: Uuid,
	automatic_match_reason: AutomaticMatchReasonEnum,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	create_or_update_signature_metadata_mapping(
		SignatureMetadataMappingInputBuilder::default()
			.provider(MetadataProviderEnum::Igdb)
			.provider_id(Some(provider_id))
			.game_id(Some(game_id))
			.match_type(MatchTypeEnum::Automatic)
			.automatic_match_reason(Some(automatic_match_reason))
			.build()?,
		db_conn,
	)
	.await?;

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
