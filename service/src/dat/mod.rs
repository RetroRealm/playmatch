use crate::constants::PARALLELISM;
use crate::dat::no_intro::download::download_no_intro_dats;
use crate::dat::redump::download::download_redump_dats;
use crate::dat::shared::import::parse_and_import_dat_file;
use crate::db::dat_file_import::is_dat_already_in_history;
use crate::db::signature_group::find_signature_group_by_name;
use crate::fs;
use crate::fs::calculate_md5;
use crate::r#match::clone::populate_all_clone_of_ids;
use anyhow::anyhow;
use fs::read_files_recursive;
use log::{debug, error, info};
use reqwest::Client;
use sea_orm::DbConn;
use std::path::PathBuf;

mod no_intro;
mod redump;
pub mod shared;

const DATS_PATH: &str = "dats";
const TMP_PATH: &str = "tmp";

pub async fn download_and_parse_dats(client: &Client, conn: &DbConn) -> anyhow::Result<()> {
	let current_dir = std::env::current_dir()?;
	let tmp_dir = current_dir.join(DATS_PATH).join(TMP_PATH);
	tokio::fs::create_dir_all(&tmp_dir).await?;

	info!("Starting to download No-Intro DATs.");
	download_no_intro_dats(client).await?;
	info!("Successfully downloaded No-Intro DATs");
	info!("Starting to download Redump DATs.");
	download_redump_dats(client).await?;
	info!("Successfully downloaded Redump DATs");

	tokio::fs::remove_dir_all(&tmp_dir).await?;

	let files = read_files_recursive(&PathBuf::from(DATS_PATH)).await?;

	let mut file_hashes = Vec::with_capacity(files.len());

	info!("Calculating MD5 hashes for DAT files, this may take a bit");
	for file_chunk in files.chunks(*PARALLELISM) {
		let mut futures = vec![];

		for file in file_chunk {
			let file = file.to_owned();
			futures.push(tokio::spawn(async move {
				let md5_hash = calculate_md5(&file).await?;

				debug!("Calculated MD5 hash for file: {:?}", file);

				Ok::<(String, PathBuf), anyhow::Error>((md5_hash, file))
			}));
		}

		for future in futures {
			let output = future.await??;

			file_hashes.push((output.0, output.1));
		}
	}
	info!("Finished calculating MD5 hashes for DAT files");

	for (hash, file) in file_hashes {
		let file_name = file
			.file_name()
			.unwrap_or_default()
			.to_str()
			.unwrap_or_default();

		let extension = file
			.extension()
			.unwrap_or_default()
			.to_str()
			.unwrap_or_default();

		if extension != "dat" || file_name.contains("BIOS") {
			debug!(
				"Skipping file: {:?}, either has no .dat file extension or contains BIOS",
				file_name
			);
			continue;
		}

		let already_imported = is_dat_already_in_history(&hash, conn).await?;

		if already_imported {
			debug!("Dat file already imported: {:?}", file);
			continue;
		}

		let path_canonical = file.canonicalize()?;

		let parent = path_canonical.to_str().unwrap_or_default();

		let mut signature_group = None;

		if parent.contains("no-intro") {
			signature_group = Some("No-Intro");
		}

		if parent.contains("redump") {
			signature_group = Some("Redump");
		}

		if parent.contains("tosec") {
			signature_group = Some("TOSEC");
		}

		if parent.contains("mame") {
			signature_group = Some("MAME");
		}

		let signature_group_entity = match signature_group {
			None => {
				return Err(anyhow!("Signature Group not found"));
			}
			Some(signature_group_name) => {
				match find_signature_group_by_name(signature_group_name, conn).await? {
					Some(sg) => sg,
					None => {
						return Err(anyhow!(
							"Signature Group not found in database (are all migrations applied?)"
						));
					}
				}
			}
		};

		if let Err(e) =
			parse_and_import_dat_file(&file, signature_group_entity.id, &hash, conn).await
		{
			error!("Failed to parse and import dat file: {:?}, {}", file, e);
		}
	}
	info!("Finished importing all DAT files");

	populate_all_clone_of_ids(conn).await?;
	info!("Finished populating all clone_of relationships");

	Ok(())
}
