use crate::db::company::create_or_find_company_by_name;
use crate::db::dat_file::{DatFileCreateOrUpdateInput, create_or_update_dat_file};
use crate::db::dat_file_import::create_dat_file_import;
use crate::db::game::{find_game_by_name_and_dat_file_id, get_game_by_id, insert_game};
use crate::db::game_file::{
	assign_content_anchor_for_game, get_game_files_from_game_id, insert_game_file_bulk,
};
use crate::db::lifecycle::reconcile_dat_file_lifecycle;
use crate::db::platform::create_or_find_platform_by_name;
use crate::identification::cache::{bust_identify_cache_for_game, bust_identify_cache_for_hashes};
use crate::ingestion::parser::model::{Datafile, Game};
use crate::ingestion::parser::regex::{DAT_PAREN_GROUP_REGEX, DAT_TAG_REGEX};
use crate::providers::content_anchor::seed_mappings_from_sibling;
use entity::{company, dat_file_import, platform};
use sea_orm::prelude::Uuid;
use std::collections::HashSet;

use crate::config::PARALLELISM;
use entity::game::Model;
use lazy_static::lazy_static;
use log::warn;
use redis::aio::MultiplexedConnection;
use regex::Regex;
use sea_orm::{ActiveModelTrait, DbConn, DbErr, IntoActiveModel, Set};
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::task;
use tokio::task::JoinHandle;

/// What a single game contributed to an import: its id, the ids of files that
/// already existed and are still present in this version, and whether this
/// import added any file (a fresh game, or new files on an existing game). A
/// pure addition retires nothing, so the lifecycle bust never fires for it; the
/// added hashes are busted separately by hash.
struct GamePresence {
	game_id: Uuid,
	present_existing_file_ids: Vec<Uuid>,
	added_files: bool,
	is_new: bool,
}

pub async fn parse_and_import_dat_file(
	path: &Path,
	signature_group_id: Uuid,
	md5_hash: &str,
	conn: &DbConn,
	redis_conn: &mut MultiplexedConnection,
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

	let mut all_game_ids: Vec<Uuid> = Vec::new();
	let mut present_existing_file_ids: Vec<Uuid> = Vec::new();
	let mut added_file_game_ids: Vec<Uuid> = Vec::new();
	let mut new_game_ids: Vec<Uuid> = Vec::new();

	if let Some(games) = dat.game {
		let games_chunked = games
			.chunks(*PARALLELISM)
			.map(|x: &[Game]| x.to_vec())
			.collect::<Vec<Vec<Game>>>();

		for game_chunk in games_chunked {
			let mut futures: Vec<JoinHandle<anyhow::Result<GamePresence>>> = vec![];

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

						// Files still present in this import. Vanished files are no
						// longer deleted; the reconciliation pass retires them so the
						// hash is never lost.
						let mut present_file_ids = Vec::new();
						for file in existing_files.iter() {
							let identifier = (
								&file.file_name,
								file.file_size_in_bytes,
								&file.crc,
								&file.md5,
								&file.sha1,
								&file.sha256,
							);
							if new_files_set.contains(&identifier) {
								present_file_ids.push(file.id);
							}
						}

						let mut to_insert = vec![];

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

						// sqlx-postgres panics past its bound-parameter limit, so inserts are chunked.
						for chunk in to_insert.chunks(*PARALLELISM) {
							insert_game_file_bulk(
								chunk.to_vec(),
								existing_game.id,
								import.id,
								&conn,
							)
							.await?;
						}

						return Ok(GamePresence {
							game_id: existing_game.id,
							present_existing_file_ids: present_file_ids,
							added_files: !to_insert.is_empty(),
							is_new: false,
						});
					}

					let game_release = insert_game(import.id, game.clone(), &conn).await?;

					// sqlx-postgres panics past its bound-parameter limit, so inserts are chunked.
					for chunk in game.rom.chunks(*PARALLELISM) {
						insert_game_file_bulk(chunk.to_vec(), game_release.id, import.id, &conn)
							.await?;
					}

					Ok(GamePresence {
						game_id: game_release.id,
						present_existing_file_ids: Vec::new(),
						added_files: true,
						is_new: true,
					})
				}));
			}

			for future in futures {
				let presence = future.await??;
				all_game_ids.push(presence.game_id);
				if presence.added_files {
					added_file_game_ids.push(presence.game_id);
				}
				if presence.is_new {
					new_game_ids.push(presence.game_id);
				}
				present_existing_file_ids.extend(presence.present_existing_file_ids);
			}
		}
	}

	let retired_game_ids = reconcile_dat_file_lifecycle(
		import.dat_file_id,
		import.id,
		&present_existing_file_ids,
		&all_game_ids,
		conn,
	)
	.await?;

	// Anchor assignment and seed-from-sibling run only after the lifecycle pass
	// has settled is_current, since the content key digests current files. A
	// per-game failure must not fail the import.
	new_game_ids.sort();
	new_game_ids.dedup();
	for game_id in &new_game_ids {
		if let Err(e) = assign_and_seed_new_game(*game_id, conn, redis_conn).await {
			warn!("Failed to assign content anchor for new game {game_id}: {e:#}");
		}
	}

	// A retired hash leaves a stale identify cache entry; a bust failure must
	// not fail the import.
	for game_id in retired_game_ids {
		if let Err(e) = bust_identify_cache_for_game(redis_conn, conn, game_id).await {
			warn!("Failed to bust identify cache for retired game {game_id}: {e}");
		}
	}

	// A pure addition retires nothing, so the loop above never touches a hash a
	// newly added game now shares with an existing one. Bust by hash across the
	// full co-hashed set so a stale single winner cannot persist for the TTL.
	added_file_game_ids.sort();
	added_file_game_ids.dedup();
	let mut added_files = Vec::new();
	for game_id in added_file_game_ids {
		added_files.extend(get_game_files_from_game_id(game_id, conn).await?);
	}
	if let Err(e) = bust_identify_cache_for_hashes(redis_conn, conn, &added_files).await {
		warn!("Failed to bust identify cache for added hashes: {e}");
	}

	Ok(())
}

/// Assign a content anchor to a freshly inserted game and, if it lands on an
/// anchor that already has mapped siblings, copy their mappings down. A game
/// left anchorless (a SHA1-less file or a vetoed engineered collision) is
/// silently skipped.
async fn assign_and_seed_new_game(
	game_id: Uuid,
	conn: &DbConn,
	redis_conn: &mut MultiplexedConnection,
) -> anyhow::Result<()> {
	let Some(game) = get_game_by_id(game_id, conn).await? else {
		return Ok(());
	};

	let Some(anchor_id) = assign_content_anchor_for_game(&game, conn).await? else {
		return Ok(());
	};

	seed_mappings_from_sibling(&game, anchor_id, conn, redis_conn).await
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

// DAT header names are not structured data. This heuristically splits a
// header into company, platform and tags, tuned against the DAT filenames
// pinned by the unit tests below.
fn parse_company_and_platform(
	dat: &Datafile,
) -> anyhow::Result<(Option<String>, String, Vec<String>)> {
	let mut dat_header = dat.header.name.clone();

	dat_header = dat_header.replace("Arcade - ", "");

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
			platform = split[0].to_string();
		}
		2 => {
			company = Some(split[0].to_string());
			platform = split[1].to_string();
		}
		_ => {
			company = Some(split[0].to_string());

			let remaining_parts = split[1..].to_vec();

			// Stop joining parts onto the platform once one looks like extra
			// metadata (bracketed tags, NKit, RVZ, and similar).
			let mut platform_parts = Vec::new();
			for part in remaining_parts {
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

	// Some GameCube DAT headers repeat the company inside the platform segment
	// ("Nintendo - NintendoGameCube"); strip it, with and without a space.
	if let Some(ref company_name) = company {
		if platform.starts_with(company_name) && platform.to_lowercase().contains("gamecube") {
			let after_company = platform.strip_prefix(company_name).unwrap_or(&platform);
			platform = after_company
				.trim_start_matches(' ')
				.trim_start_matches('-')
				.trim()
				.to_string();
		}

		let company_no_spaces = company_name.replace(' ', "");
		if platform.starts_with(&company_no_spaces) && platform.len() > company_no_spaces.len() {
			let potential_platform = &platform[company_no_spaces.len()..];
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

	platform = platform.replace(&format!(" ({version})"), "");

	let mut clean_platform = platform.clone();
	for capture in DAT_TAG_REGEX.captures_iter(&platform) {
		if let Some(tag_match) = capture.get(1) {
			let tag = tag_match.as_str();
			// Skip version-like tokens such as "2023-01-01" so they are not treated as tags.
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

		platform = format!("PlayStation {number}");
	}

	Ok((company, platform, tags))
}

pub fn sanitize_dat_string(mut file_name: String, file_extension: &str, version: &str) -> String {
	file_name = file_name.replace(format!(" ({version})").as_str(), "");

	file_name = DAT_PAREN_GROUP_REGEX
		.replace_all(&file_name, |caps: &regex::Captures| {
			if is_build_stamp(&caps[1]) {
				String::new()
			} else {
				caps[0].to_string()
			}
		})
		.into_owned();

	file_name = file_name.replace(format!(".{file_extension}").as_str(), "");

	file_name
}

/// True for build stamps and release counts (digits and separators only, such
/// as "(20260605-234217)" or "(100)"); they change per build and would
/// otherwise fork a new dat file row each time. Letter-bearing tags like
/// (Decrypted) stay.
fn is_build_stamp(inner: &str) -> bool {
	inner.chars().any(|c| c.is_ascii_digit())
		&& inner
			.chars()
			.all(|c| c.is_ascii_digit() || matches!(c, '-' | '_' | ' ' | ':'))
}

/// Maximum DAT file size accepted by `parse_dat_file`. Largest observed real
/// DAT is ~78 MiB; 125 MiB gives headroom without unbounded heap growth on
/// malformed or hostile input.
const MAX_DAT_BYTES: u64 = 125 * 1024 * 1024;

pub async fn parse_dat_file(path: &Path) -> anyhow::Result<Datafile> {
	let meta = tokio::fs::metadata(path).await?;
	if meta.len() > MAX_DAT_BYTES {
		anyhow::bail!(
			"DAT file {} is {} bytes, exceeds MAX_DAT_BYTES ({})",
			path.display(),
			meta.len(),
			MAX_DAT_BYTES,
		);
	}

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

	#[test]
	fn sanitize_strips_matching_version_stamp() {
		let result = sanitize_dat_string(
			"Nintendo - Nintendo DS (Decrypted) (20260617-122122).dat".to_string(),
			"dat",
			"20260617-122122",
		);
		assert_eq!(result, "Nintendo - Nintendo DS (Decrypted)");
	}

	#[test]
	fn sanitize_strips_mismatching_version_stamp() {
		// Filename stamp differs from the header version.
		let result = sanitize_dat_string(
			"Nintendo - Nintendo DS (Decrypted) (20260605-234217).dat".to_string(),
			"dat",
			"20260605-170618",
		);
		assert_eq!(result, "Nintendo - Nintendo DS (Decrypted)");
	}

	#[test]
	fn sanitize_strips_pure_count_tag() {
		let spaced = sanitize_dat_string(
			"Sony - PlayStation - Discs (100).dat".to_string(),
			"dat",
			"2026-01-01",
		);
		assert_eq!(spaced, "Sony - PlayStation - Discs");

		let with_count_and_stamp = sanitize_dat_string(
			"Nintendo - Wii U - WUX (511) (20220906-192735).dat".to_string(),
			"dat",
			"2022-09-06 19:27:35",
		);
		assert_eq!(with_count_and_stamp, "Nintendo - Wii U - WUX");
	}

	#[test]
	fn sanitize_strips_colon_and_space_timestamp() {
		let result = sanitize_dat_string(
			"Nintendo - GameCube (2023-01-09 15:43:45).dat".to_string(),
			"dat",
			"20230109-154345",
		);
		assert_eq!(result, "Nintendo - GameCube");
	}

	#[test]
	fn sanitize_collapses_doubled_stamp() {
		let result = sanitize_dat_string(
			"Nintendo - Nintendo DS (Decrypted) (20260605-234217) (20260605-234217).dat"
				.to_string(),
			"dat",
			"20260605-170618",
		);
		assert_eq!(result, "Nintendo - Nintendo DS (Decrypted)");
	}

	#[test]
	fn sanitize_preserves_decrypted_and_encrypted_distinct() {
		let decrypted = sanitize_dat_string(
			"Nintendo - Nintendo DS (Decrypted) (20260605-234217).dat".to_string(),
			"dat",
			"20260605-170618",
		);
		let encrypted = sanitize_dat_string(
			"Nintendo - Nintendo DS (Encrypted) (20260605-234217).dat".to_string(),
			"dat",
			"20260605-170618",
		);
		assert_eq!(decrypted, "Nintendo - Nintendo DS (Decrypted)");
		assert_eq!(encrypted, "Nintendo - Nintendo DS (Encrypted)");
		assert_ne!(decrypted, encrypted);
	}

	#[test]
	fn sanitize_preserves_headered_headerless_and_letter_digit_tags() {
		let headered = sanitize_dat_string(
			"Nintendo - Nintendo Entertainment System (Headered) (20260422-002744).dat".to_string(),
			"dat",
			"20260421-122918",
		);
		let headerless = sanitize_dat_string(
			"Nintendo - Nintendo Entertainment System (Headerless) (20260422-002744).dat"
				.to_string(),
			"dat",
			"20260421-122918",
		);
		assert_eq!(
			headered,
			"Nintendo - Nintendo Entertainment System (Headered)"
		);
		assert_eq!(
			headerless,
			"Nintendo - Nintendo Entertainment System (Headerless)"
		);
		assert_ne!(headered, headerless);

		for tag in ["(A2R)", "(A78)", "(J64)", "(PSX2PSP)"] {
			let name = format!("Acme - Platform {tag}");
			let sanitized = sanitize_dat_string(format!("{name}.dat"), "dat", "2026-01-01");
			assert_eq!(sanitized, name, "tag {tag} must be preserved");
		}
	}
}
