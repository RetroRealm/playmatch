use crate::db::company::create_or_find_company_by_name;
use crate::db::dat_file::{DatFileCreateOrUpdateInput, create_or_update_dat_file};
use crate::db::dat_file_import::create_dat_file_import;
use crate::db::game::{find_game_by_name_and_dat_file_id, insert_game};
use crate::db::game_file::{get_game_files_from_game_id, insert_game_file_bulk};
use crate::db::platform::create_or_find_platform_by_name;
use crate::ingestion::parser::model::{Datafile, Game};
use crate::ingestion::parser::regex::{DAT_NUMBER_REGEX, DAT_TAG_REGEX};
use entity::{company, dat_file_import, platform};
use sea_orm::prelude::Uuid;
use std::collections::HashSet;

use crate::constants::PARALLELISM;
use entity::game::Model;
use lazy_static::lazy_static;
use regex::Regex;
use sea_orm::{ActiveModelTrait, DbConn, DbErr, IntoActiveModel, Set};
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::task;
use tokio::task::JoinHandle;

pub async fn parse_and_import_dat_file(
	path: &Path,
	signature_group_id: Uuid,
	md5_hash: &str,
	conn: &DbConn,
) -> anyhow::Result<()> {
	let dat = parse_dat_file(path).await?;

	let (company, system, tags) = match parse_company_and_platform(&dat) {
		Ok(value) => value,
		Err(err) => return Err(err),
	};

	let file_name = path
		.file_name()
		.unwrap_or_default()
		.to_str()
		.unwrap_or_default();

	let file_extension = path
		.extension()
		.unwrap_or_default()
		.to_str()
		.unwrap_or_default();

	let sanitized_file_name =
		sanitize_dat_string(file_name.to_string(), file_extension, &dat.header.version);

	let (company, platform) = insert_or_get_company_and_platform(company, &system, conn).await?;
	let import = update_dat_file_and_insert_dat_file_import(
		DatFileCreateOrUpdateInput {
			signature_group_id,
			sanitized_file_name,
			current_version: dat.header.version.clone(),
			tags,
			subset: dat.header.subset.clone(),
			company_id: company.clone().map(|c| c.id),
			platform_id: platform.id,
		},
		file_name,
		md5_hash,
		conn,
	)
	.await?;

	if let Some(games) = dat.game {
		let games_chunked = games
			.chunks(*PARALLELISM)
			.map(|x: &[Game]| x.to_vec())
			.collect::<Vec<Vec<Game>>>();

		for game_chunk in games_chunked {
			let mut futures: Vec<JoinHandle<anyhow::Result<()>>> = vec![];

			for game in game_chunk {
				let conn = conn.clone();
				let import = import.clone();
				futures.push(task::spawn(async move {
					let result =
						find_game_by_name_and_dat_file_id(&game.name, import.dat_file_id, &conn)
							.await?;

					if let Some(existing_game) = result {
						update_game_properties(&game, &conn, &existing_game).await?;

						let existing_files =
							get_game_files_from_game_id(existing_game.id, &conn).await?;
						let existing_files_set: HashSet<_> = existing_files
							.iter()
							.map(|file| {
								(
									&file.file_name,
									file.file_size_in_bytes,
									&file.crc,
									&file.md5,
									&file.sha1,
									&file.sha256,
								)
							})
							.collect();

						let new_files_set: HashSet<_> = game
							.rom
							.iter()
							.map(|rom| {
								(
									&rom.name,
									rom.size.as_ref().and_then(|s| s.parse::<i64>().ok()),
									&rom.crc,
									&rom.md5,
									&rom.sha1,
									&rom.sha256,
								)
							})
							.collect();

						// Delete existing files that are not in the new files
						for file in existing_files.iter() {
							let identifier = (
								&file.file_name,
								file.file_size_in_bytes,
								&file.crc,
								&file.md5,
								&file.sha1,
								&file.sha256,
							);
							if !new_files_set.contains(&identifier) {
								file.clone().into_active_model().delete(&conn).await?;
							}
						}

						let mut to_insert = vec![];

						// Insert new files that are not in the existing files
						for rom in game.rom.iter() {
							let identifier = (
								&rom.name,
								rom.size.as_ref().and_then(|s| s.parse::<i64>().ok()),
								&rom.crc,
								&rom.md5,
								&rom.sha1,
								&rom.sha256,
							);
							if !existing_files_set.contains(&identifier) {
								to_insert.push(rom.clone());
							}
						}

						// When we insert too many sqlx-postgres panics, so we chunk the inserts
						for chunk in to_insert.chunks(*PARALLELISM) {
							insert_game_file_bulk(chunk.to_vec(), existing_game.id, &conn).await?;
						}

						return Ok(());
					}

					let game_release = insert_game(import.id, game.clone(), &conn).await?;

					// When we insert too many sqlx-postgres panics, so we chunk the inserts
					for chunk in game.rom.chunks(*PARALLELISM) {
						insert_game_file_bulk(chunk.to_vec(), game_release.id, &conn).await?;
					}

					Ok(())
				}));
			}

			for future in futures {
				future.await??;
			}
		}
	}

	Ok(())
}

async fn update_game_properties(
	game: &Game,
	conn: &DbConn,
	existing_game: &Model,
) -> Result<(), DbErr> {
	let mut has_changes = false;

	let mut existing_game_active = existing_game.clone().into_active_model();

	if existing_game.signature_group_internal_id != game.id {
		existing_game_active.signature_group_internal_id = Set(game.id.clone());
		has_changes = true;
	}

	if existing_game.signature_group_internal_clone_of_id != game.cloneofid {
		existing_game_active.signature_group_internal_clone_of_id = Set(game.cloneofid.clone());
		has_changes = true;
	}

	if existing_game.description != game.description {
		existing_game_active.description = Set(game.description.clone());
		has_changes = true;
	}

	if existing_game.categories != game.category {
		existing_game_active.categories = Set(game.category.clone());
		has_changes = true;
	}

	if has_changes {
		existing_game_active.save(conn).await?;
	}
	Ok(())
}

fn parse_company_and_platform(
	dat: &Datafile,
) -> anyhow::Result<(Option<String>, String, Vec<String>)> {
	let mut dat_header = dat.header.name.clone();

	// Remove Arcade - from the name as its not a company or system
	dat_header = dat_header.replace("Arcade - ", "");

	// Remove subset prefix if present
	if let Some(subset) = &dat.header.subset {
		let subset_prefix = format!("{subset} - ");
		if dat_header.starts_with(&subset_prefix) {
			dat_header = dat_header.replacen(&subset_prefix, "", 1);
		}
	}

	let split = dat_header.split(" - ").collect::<Vec<&str>>();

	if split.is_empty() || (split.len() == 1 && split[0].is_empty()) {
		return Err(anyhow::anyhow!("No company or system found"));
	}

	let version = &dat.header.version;
	let mut tags = Vec::new();
	let mut company = None;
	let mut platform;

	match split.len() {
		1 => {
			// Single part - it's the platform, no company
			platform = split[0].to_string();
		}
		2 => {
			// Two parts - company and platform
			company = Some(split[0].to_string());
			platform = split[1].to_string();
		}
		_ => {
			// Three or more parts
			company = Some(split[0].to_string());

			// Join the remaining parts
			let remaining_parts = split[1..].to_vec();

			// Check if this looks like extra metadata (contains brackets, "NKit", etc)
			let mut platform_parts = Vec::new();
			for part in remaining_parts {
				// Stop adding to platform if we hit what looks like metadata
				if part.contains('[')
					|| part.contains("NKit")
					|| part.contains("RVZ")
					|| part.contains("Discs")
					|| part.contains("zstd")
					|| part.contains("WUX")
				{
					break;
				}
				platform_parts.push(part);
			}

			platform = if platform_parts.is_empty() {
				split[1].to_string()
			} else {
				platform_parts.join(" - ")
			};
		}
	}

	// Remove company name from platform if it appears there for GameCube
	if let Some(ref company_name) = company {
		// Remove "Company Platform" -> "Platform"
		if platform.starts_with(company_name) && platform.to_lowercase().contains("gamecube") {
			let after_company = platform.strip_prefix(company_name).unwrap_or(&platform);
			// Clean up any leading spaces or separators
			platform = after_company
				.trim_start_matches(' ')
				.trim_start_matches('-')
				.trim()
				.to_string();
		}

		// Also check for "CompanyPlatform" (no space) patterns
		let company_no_spaces = company_name.replace(' ', "");
		if platform.starts_with(&company_no_spaces) && platform.len() > company_no_spaces.len() {
			let potential_platform = &platform[company_no_spaces.len()..];
			// Check if the next character is uppercase (indicating camelCase split)
			if potential_platform
				.chars()
				.next()
				.map(|c| c.is_uppercase())
				.unwrap_or(false)
			{
				platform = potential_platform.to_string();
			}
		}
	}

	// Remove version from platform if present
	platform = platform.replace(&format!(" ({version})"), "");

	// Extract tags from platform
	let mut clean_platform = platform.clone();
	for capture in DAT_TAG_REGEX.captures_iter(&platform) {
		if let Some(tag_match) = capture.get(1) {
			let tag = tag_match.as_str();
			// Don't treat version-like strings as tags
			if !tag.contains('-') || !tag.chars().all(|c| c.is_numeric() || c == '-' || c == ' ') {
				tags.push(tag.to_owned());
				clean_platform = clean_platform.replace(&format!(" ({tag})"), "");
			}
		}
	}
	platform = clean_platform.trim().to_string();

	lazy_static! {
		// Matches "PS" followed by digits, possibly followed by " - anything"
		static ref PS_REGEX: Regex = Regex::new(r"^PS(\d+)(?:\s*-\s*.*)?$").unwrap();
	}

	if let Some(captures) = PS_REGEX.captures(&platform)
		&& let Some(number_match) = captures.get(1)
	{
		let number_str = number_match.as_str();
		let number = number_str.parse::<u32>().unwrap_or(1);

		// Convert PS1, PS2, PS3, etc. to PlayStation 1, PlayStation 2, etc.
		platform = format!("PlayStation {number}");
	}

	Ok((company, platform, tags))
}

pub fn sanitize_dat_string(mut file_name: String, file_extension: &str, version: &str) -> String {
	file_name = file_name.replace(format!(" ({version})").as_str(), "");

	for tag in DAT_NUMBER_REGEX.captures_iter(&file_name.clone()) {
		let tag = tag.get(0).map(|x| x.as_str()).unwrap_or_default();
		file_name = file_name.replace(&format!(" {tag}"), "");
	}

	file_name = file_name.replace(format!(".{file_extension}").as_str(), "");

	file_name
}

pub async fn parse_dat_file(path: &Path) -> anyhow::Result<Datafile> {
	let mut dat_file = File::open(path).await?;

	let mut content = Vec::new();
	dat_file.read_to_end(&mut content).await?;

	let result: Datafile = serde_xml_rs::from_reader(content.as_slice())?;

	Ok(result)
}

pub async fn insert_or_get_company_and_platform(
	company_name: Option<String>,
	platform_name: &str,
	conn: &DbConn,
) -> anyhow::Result<(Option<company::Model>, platform::Model)> {
	let company = if let Some(company_name) = &company_name {
		Some(create_or_find_company_by_name(company_name.as_str(), conn).await?)
	} else {
		None
	};

	let platform =
		create_or_find_platform_by_name(platform_name, company.clone().map(|c| c.id), conn).await?;

	Ok((company, platform))
}

pub async fn update_dat_file_and_insert_dat_file_import(
	input: DatFileCreateOrUpdateInput,
	original_file_name: &str,
	md5_hash: &str,
	conn: &DbConn,
) -> anyhow::Result<dat_file_import::Model> {
	let current_version = input.current_version.clone();
	let dat_file = create_or_update_dat_file(input, conn).await?;

	Ok(create_dat_file_import(
		original_file_name,
		md5_hash,
		&current_version,
		dat_file.id,
		conn,
	)
	.await?)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::ingestion::parser::model::Header;

	fn create_datafile(name: &str, subset: Option<&str>, version: &str) -> Datafile {
		Datafile {
			header: Header {
				id: None,
				name: name.to_string(),
				subset: subset.map(|s| s.to_string()),
				author: None,
				homepage: None,
				version: version.to_string(),
				description: None,
				url: None,
			},
			game: None,
		}
	}

	#[test]
	fn test_nintendo_gamecube_non_redump() {
		let dat = create_datafile(
			"Non-Redump - Nintendo - Nintendo GameCube",
			Some("Non-Redump"),
			"20250405-114402",
		);

		let result = parse_company_and_platform(&dat).unwrap();
		assert_eq!(result.0, Some("Nintendo".to_string()));
		assert_eq!(result.1, "GameCube".to_string());
		assert_eq!(result.2, Vec::<String>::new());
	}

	#[test]
	fn test_nintendo_gamecube_redump() {
		let dat = create_datafile("Nintendo - GameCube", None, "2025-07-11 07-41-21");

		let result = parse_company_and_platform(&dat).unwrap();
		assert_eq!(result.0, Some("Nintendo".to_string()));
		assert_eq!(result.1, "GameCube".to_string());
		assert_eq!(result.2, Vec::<String>::new());
	}

	#[test]
	fn test_nintendo_gamecube_nkit() {
		let dat = create_datafile(
			"Nintendo - GameCube - NKit RVZ [zstd-19-128k]",
			None,
			"2023-01-09 15:43:45",
		);

		let result = parse_company_and_platform(&dat).unwrap();
		assert_eq!(result.0, Some("Nintendo".to_string()));
		assert_eq!(result.1, "GameCube".to_string());
		assert_eq!(result.2, Vec::<String>::new());
	}

	#[test]
	fn test_arcade_sega_naomi() {
		let dat = create_datafile("Arcade - Sega - Naomi", None, "2025-03-30 19-07-31");

		let result = parse_company_and_platform(&dat).unwrap();
		assert_eq!(result.0, Some("Sega".to_string()));
		assert_eq!(result.1, "Naomi".to_string());
		assert_eq!(result.2, Vec::<String>::new());
	}

	#[test]
	fn test_microsoft_xbox() {
		let dat = create_datafile("Microsoft - Xbox", None, "2025-07-18 20-42-57");

		let result = parse_company_and_platform(&dat).unwrap();
		assert_eq!(result.0, Some("Microsoft".to_string()));
		assert_eq!(result.1, "Xbox".to_string());
		assert_eq!(result.2, Vec::<String>::new());
	}

	#[test]
	fn test_sony_playstation3() {
		let dat = create_datafile(
			"Unofficial - Sony - PlayStation 3 (BD-Video Extras)",
			Some("Unofficial"),
			"20250405-202040",
		);

		let result = parse_company_and_platform(&dat).unwrap();
		assert_eq!(result.0, Some("Sony".to_string()));
		assert_eq!(result.1, "PlayStation 3".to_string());
		assert_eq!(result.2, vec!["BD-Video Extras".to_string()]);
	}

	#[test]
	fn test_sega_mega_drive_genesis() {
		let dat = create_datafile("Sega - Mega Drive - Genesis", None, "20250715-223313");

		let result = parse_company_and_platform(&dat).unwrap();
		assert_eq!(result.0, Some("Sega".to_string()));
		assert_eq!(result.1, "Mega Drive - Genesis".to_string());
		assert_eq!(result.2, Vec::<String>::new());
	}

	#[test]
	fn test_sony_playstation_portable_psn() {
		let dat = create_datafile(
			"Sony - PlayStation Portable (PSN) (Decrypted)",
			None,
			"20250717-231452",
		);

		let result = parse_company_and_platform(&dat).unwrap();
		assert_eq!(result.0, Some("Sony".to_string()));
		assert_eq!(result.1, "PlayStation Portable".to_string());
		assert_eq!(result.2, vec!["PSN".to_string(), "Decrypted".to_string()]);
	}

	#[test]
	fn test_platform_with_tags() {
		let dat = create_datafile("Nintendo - GameCube (Demo) (Beta)", None, "2023-01-01");

		let result = parse_company_and_platform(&dat).unwrap();
		assert_eq!(result.0, Some("Nintendo".to_string()));
		assert_eq!(result.1, "GameCube".to_string());
		assert_eq!(result.2, vec!["Demo".to_string(), "Beta".to_string()]);
	}

	#[test]
	fn test_no_company() {
		let dat = create_datafile("GameCube", None, "2023-01-01");

		let result = parse_company_and_platform(&dat).unwrap();
		assert_eq!(result.0, None);
		assert_eq!(result.1, "GameCube".to_string());
		assert_eq!(result.2, Vec::<String>::new());
	}

	#[test]
	fn test_empty_header() {
		let dat = create_datafile("", None, "2023-01-01");

		let result = parse_company_and_platform(&dat);
		assert!(result.is_err());
	}

	#[test]
	fn test_version_in_platform_name() {
		let dat = create_datafile("Company - Platform (2023-01-01)", None, "2023-01-01");

		let result = parse_company_and_platform(&dat).unwrap();
		assert_eq!(result.0, Some("Company".to_string()));
		assert_eq!(result.1, "Platform".to_string());
		assert_eq!(result.2, Vec::<String>::new());
	}

	#[test]
	fn test_complex_metadata_suffix() {
		let dat = create_datafile(
			"Sony - PlayStation - NKit RVZ [zstd-19-128k] - Discs (100)",
			None,
			"2023-01-01",
		);

		let result = parse_company_and_platform(&dat).unwrap();
		assert_eq!(result.0, Some("Sony".to_string()));
		assert_eq!(result.1, "PlayStation".to_string());
		assert_eq!(result.2, Vec::<String>::new());
	}

	#[test]
	fn test_playstation_exception() {
		let dat = create_datafile("Sony - PS3 - Decrypted", None, "2022-09-18 03:49:47");

		let result = parse_company_and_platform(&dat).unwrap();
		assert_eq!(result.0, Some("Sony".to_string()));
		assert_eq!(result.1, "PlayStation 3".to_string());
		assert_eq!(result.2, Vec::<String>::new());
	}

	#[test]
	fn test_wii_u_wux_naming() {
		let dat = create_datafile("Nintendo - Wii U - WUX", None, "2022-09-06 19:27:35");

		let result = parse_company_and_platform(&dat).unwrap();
		assert_eq!(result.0, Some("Nintendo".to_string()));
		assert_eq!(result.1, "Wii U".to_string());
		assert_eq!(result.2, Vec::<String>::new());
	}

	#[test]
	fn test_game_boy_color_source_code_naming() {
		let dat = create_datafile(
			"Source Code - Nintendo - Game Boy Color",
			Some("Source Code"),
			"20250313-191959",
		);

		let result = parse_company_and_platform(&dat).unwrap();
		assert_eq!(result.0, Some("Nintendo".to_string()));
		assert_eq!(result.1, "Game Boy Color".to_string());
		assert_eq!(result.2, Vec::<String>::new());
	}

	#[test]
	fn test_game_boy_color_naming() {
		let dat = create_datafile("Nintendo - Game Boy Color", None, "20250614-001651");

		let result = parse_company_and_platform(&dat).unwrap();
		assert_eq!(result.0, Some("Nintendo".to_string()));
		assert_eq!(result.1, "Game Boy Color".to_string());
		assert_eq!(result.2, Vec::<String>::new());
	}
}
