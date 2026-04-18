use crate::automatic_match::PAGE_SIZE;
use crate::providers::igdb::matching::{IGDB_CHUNK_SIZE, clean_name};
use crate::automatic_match::util::roman_to_int;
use crate::db::game::{
	find_game_parent, find_game_signature_metadata_mapping,
	get_automatic_match_failed_games_with_limit, get_unmatched_games_with_clone_of_with_limit,
	get_unmatched_games_without_clone_of_with_limit,
};
use crate::db::platform::{
	find_platform_of_game, find_platform_related_signature_metadata_mapping,
};
use crate::db::signature_metadata_mapping::{
	SignatureMetadataMappingInputBuilder, create_or_update_signature_metadata_mapping,
};
use crate::providers::igdb::IgdbClient;
use entity::game::Model;
use entity::sea_orm_active_enums::{
	AutomaticMatchReasonEnum, FailedMatchReasonEnum, MatchTypeEnum, MetadataProviderEnum,
};
use futures_util::future::BoxFuture;
use lazy_static::lazy_static;
use log::{debug, error};
use regex::Regex;
use sea_orm::DbConn;
use sea_orm::prelude::Uuid;
use std::pin::Pin;
use std::sync::Arc;

type FetchFn =
	fn(
		u64,
		DbConn,
	) -> Pin<Box<dyn Future<Output = Result<Option<Vec<Model>>, anyhow::Error>> + Send>>;

type MatchFn = fn(
	Model,
	Arc<IgdbClient>,
	DbConn,
) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + Send>>;

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

	match_games_in_batches(
		get_automatic_match_failed_games_with_limit,
		match_game_to_igdb,
		igdb_client.clone(),
		db_conn,
	)
	.await?;
	debug!("Finished matching games which failed to match 60 days ago to IGDB");

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
				results.push(tokio::spawn(match_fn(game, igdb_client.clone(), db_conn)));
			}

			for result in results {
				if let Err(e) = result.await? {
					error!("Error while matching to IGDB: {e:?}");
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
	// Basic idea, first check if the parent game is matched to IGDB,
	// if yes, then we match to the same igdb id,
	// otherwise we try to match the game to igdb, if it succeeds, we apply the same igdb to the parent game

	Box::pin(async move {
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

				create_or_update_signature_metadata_mapping_success(
					parent_game_igdb_mapping.provider_id.clone().unwrap(),
					game.id,
					AutomaticMatchReasonEnum::ViaParent,
					&db_conn,
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
				create_or_update_signature_metadata_mapping_success(
					search_result.id.to_string(),
					game.id,
					AutomaticMatchReasonEnum::DirectName,
					&db_conn,
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
				create_or_update_signature_metadata_mapping_success(
					search_result.id.to_string(),
					game.id,
					AutomaticMatchReasonEnum::NormalizedName,
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
					let alternative_name_lower = alternative_name.name.to_lowercase();

					if alternative_name_lower == clean_name {
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

					let alternative_name_normalized = normalize_title(&alternative_name_lower);

					if alternative_name_normalized == clean_name_normalized {
						debug!(
							"Matched Game \"{}\" to IGDB Game ID {} (Normalized Alternative Name Match)",
							&clean_name, search_result.id
						);
						create_or_update_signature_metadata_mapping_success(
							search_result.id.to_string(),
							game.id,
							AutomaticMatchReasonEnum::NormalizedAlternativeName,
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

pub fn normalize_title(input: &str) -> String {
	lazy_static! {
		static ref RE_STRIP: Regex = Regex::new(r" - |: ").unwrap();
		static ref RE_LEADING: Regex = Regex::new(r"^(?i)(the |a |an )").unwrap();
		// Remove ", The", ", A", ", An" anywhere in the string (case-insensitive)
		static ref RE_ARTICLE: Regex = Regex::new(r"(?i),\s*(the|a|an)\b").unwrap();
		static ref RE_ROMAN: Regex = Regex::new(
			r"\b(?i:M{0,4}(CM|CD|D?C{0,3})(XC|XL|L?X{0,3})(IX|IV|V?I{0,3}))\b"
		).unwrap();
	}

	// 1. Strip " - " and ": "
	let mut s = RE_STRIP.replace_all(input, " ").to_string();

	// 2. Strip leading article
	s = RE_LEADING.replace(&s, "").to_string();

	// 3. Remove all article suffixes (anywhere in string)
	s = RE_ARTICLE.replace_all(&s, "").to_string();

	// 4. Replace all roman numerals
	s = RE_ROMAN
		.replace_all(&s, |caps: &regex::Captures| {
			let roman = &caps[0];
			if !roman.is_empty()
				&& let Some(val) = roman_to_int(roman)
			{
				return val.to_string();
			}
			roman.to_string()
		})
		.to_string();

	s.split_whitespace().collect::<Vec<_>>().join(" ")
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
		match find_platform_related_signature_metadata_mapping(&platform, db_conn).await? {
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_normalize_title() {
		assert_eq!(
			normalize_title("The Legend of Zelda: Majora's Mask"),
			"Legend of Zelda Majora's Mask"
		);
		assert_eq!(
			normalize_title("Star Wars IV: A New Hope"),
			"Star Wars 4 A New Hope"
		);
		assert_eq!(
			normalize_title("A Series of Unfortunate Events, The"),
			"Series of Unfortunate Events"
		);
		assert_eq!(
			normalize_title("An American Tail - Fievel Goes West"),
			"American Tail Fievel Goes West"
		);
		assert_eq!(normalize_title("Final Fantasy VII"), "Final Fantasy 7");
		assert_eq!(normalize_title("Rocky II"), "Rocky 2");
		assert_eq!(normalize_title("An Untitled Story"), "Untitled Story");
		assert_eq!(
			normalize_title("Legend of Zelda, The - Twilight Princess"),
			"Legend of Zelda Twilight Princess"
		);
		assert_eq!(
			normalize_title("The Legend of Zelda: Twilight Princess"),
			"Legend of Zelda Twilight Princess"
		)
	}
}
