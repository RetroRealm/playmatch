use crate::db::game::{
	find_game_parent, find_game_signature_metadata_mapping,
	get_unmatched_games_with_clone_of_with_limit, get_unmatched_games_without_clone_of_with_limit,
};
use crate::db::platform::{find_platform_of_game, find_related_signature_metadata_mapping};
use crate::db::signature_metadata_mapping::{
	create_or_update_signature_metadata_mapping, SignatureMetadataMappingInputBuilder,
};
use crate::metadata::igdb::IgdbClient;
use crate::r#match::igdb::{clean_name, IGDB_CHUNK_SIZE};
use crate::r#match::PAGE_SIZE;
use entity::game::Model;
use entity::sea_orm_active_enums::{
	AutomaticMatchReasonEnum, FailedMatchReasonEnum, MatchTypeEnum, MetadataProviderEnum,
};
use futures_util::future::BoxFuture;
use log::{debug, error};
use sea_orm::prelude::Uuid;
use sea_orm::DbConn;
use std::pin::Pin;
use std::sync::Arc;

type FetchFn = fn(
	u64,
	DbConn,
) -> Pin<
	Box<dyn futures_util::Future<Output = Result<Option<Vec<Model>>, anyhow::Error>> + Send>,
>;

type MatchFn = fn(
	Model,
	Arc<IgdbClient>,
	DbConn,
) -> Pin<Box<dyn futures_util::Future<Output = Result<(), anyhow::Error>> + Send>>;

pub async fn match_games_to_igdb(
	igdb_client: Arc<IgdbClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	match_games_in_batches(
		get_unmatched_games_without_clone_of_with_limit,
		match_game_to_igdb,
		igdb_client.clone(),
		db_conn,
	)
	.await?;
	debug!("Finished matching games without clone_of id to IGDB");

	match_games_in_batches(
		get_unmatched_games_with_clone_of_with_limit,
		match_clone_of_game_to_igdb,
		igdb_client.clone(),
		db_conn,
	)
	.await?;
	debug!("Finished matching games with clone_of id to IGDB");

	Ok(())
}

pub async fn match_games_in_batches(
	fetch_fn: FetchFn,
	match_fn: MatchFn,
	igdb_client: Arc<IgdbClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	while let Some(page) = fetch_fn(PAGE_SIZE, db_conn.clone()).await? {
		for page_chunks in page.chunks(IGDB_CHUNK_SIZE) {
			let mut results = vec![];

			for game in page_chunks.iter().cloned() {
				let igdb_client = igdb_client.clone();
				let db_conn = db_conn.clone();
				results.push(tokio::spawn({
					match_fn(game, igdb_client.clone(), db_conn)
				}));
			}

			for result in results {
				if let Err(e) = result.await? {
					error!("Error while matching to IGDB: {:?}", e);
				}
			}
		}
	}

	Ok(())
}

fn match_clone_of_game_to_igdb<'a>(
	game: Model,
	igdb_client: Arc<IgdbClient>,
	db_conn: DbConn,
) -> BoxFuture<'a, anyhow::Result<()>> {
	// Basic idea, first check if parent game is matched to IGDB,
	// if yes then we match to the same igdb id,
	// otherwise we try to match the game to igdb, if it succeeds we apply the same igdb to the parent game

	Box::pin(async move {
		let parent_game = find_game_parent(&game, &db_conn).await?;

		if let Some(parent_game) = parent_game {
			let parent_game_igdb_mapping =
				find_game_signature_metadata_mapping(&parent_game, &db_conn).await?;

			if let Some(parent_game_igdb_mapping) = &parent_game_igdb_mapping {
				if parent_game_igdb_mapping.match_type == MatchTypeEnum::Automatic
					|| parent_game_igdb_mapping.match_type == MatchTypeEnum::Manual
				{
					debug!(
						"Matched Game \"{}\" to IGDB Game ID {} (Via Parent)",
						&game.name,
						parent_game_igdb_mapping.provider_id.clone().unwrap()
					);

					create_or_update_signature_metadata_mapping_success(
						parent_game_igdb_mapping.provider_id.clone().unwrap(),
						game.id,
						AutomaticMatchReasonEnum::ViaParent,
						&db_conn,
					)
					.await?;

					return Ok(());
				}
			}

			match_game_to_igdb(game.clone(), igdb_client.clone(), db_conn.clone()).await?;

			let mapping = find_game_signature_metadata_mapping(&game, &db_conn).await?;

			if let Some(mapping) = mapping {
				if mapping.match_type == MatchTypeEnum::Automatic
					|| mapping.match_type == MatchTypeEnum::Manual
				{
					debug!("Matched Game with parent which is not matched, overriding parent mapping... (Via Child)");

					create_or_update_signature_metadata_mapping_success(
						mapping.provider_id.unwrap(),
						parent_game.id,
						AutomaticMatchReasonEnum::ViaChild,
						&db_conn,
					)
					.await?;

					return Ok(());
				}
			}
		}

		Ok(())
	})
}

fn match_game_to_igdb<'a>(
	game: Model,
	igdb_client: Arc<IgdbClient>,
	db_conn: DbConn,
) -> BoxFuture<'a, anyhow::Result<()>> {
	Box::pin(async move {
		let platform_igdb_id = get_game_platform_igdb_id(&game, &db_conn).await?;

		let clean_name = clean_name(&game.name);

		let search_results = igdb_client
			.search_game_by_name_and_platform(&clean_name, platform_igdb_id)
			.await?;

		for search_result in search_results {
			if search_result.name.to_lowercase() == clean_name.to_lowercase() {
				debug!(
					"Matched Game \"{}\" to IGDB Game ID {} (Direct Match)",
					&clean_name, search_result.id
				);
				create_or_update_signature_metadata_mapping_success(
					search_result.id.to_string(),
					game.id,
					AutomaticMatchReasonEnum::DirectName,
					&db_conn,
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
					if alternative_name.name.to_lowercase() == clean_name.to_lowercase() {
						debug!(
							"Matched Game \"{}\" to IGDB Game ID {} (Alternative Name Match)",
							&clean_name, search_result.id
						);
						create_or_update_signature_metadata_mapping_success(
							search_result.id.to_string(),
							game.id,
							AutomaticMatchReasonEnum::AlternativeName,
							&db_conn,
						)
						.await?;

						return Ok(());
					}
				}
			}
		}

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

		Ok(())
	})
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
