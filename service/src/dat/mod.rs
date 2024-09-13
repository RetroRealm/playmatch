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

pub async fn download_and_parse_dats(client: &Client, conn: &DbConn) -> anyhow::Result<()> {
	info!("Starting to download No-Intro DATs.");
	download_no_intro_dats(client).await?;
	info!("Successfully downloaded No-Intro DATs");
	info!("Starting to download Redump DATs.");
	download_redump_dats(client).await?;
	info!("Successfully downloaded Redump DATs");

	let files = read_files_recursive(&PathBuf::from(DATS_PATH)).await?;

	for file in files {
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

		if extension != "dat" || file_name.contains("BIOS") {
			continue;
		}

		let md5_hash = calculate_md5(&file).await?;

		let already_imported = is_dat_already_in_history(&md5_hash, conn).await?;

		if already_imported {
			debug!("Dat file already imported: {:?}", file);
			continue;
		}

		if let Err(e) =
			parse_and_import_dat_file(&file, signature_group_entity.id, &md5_hash, conn).await
		{
			error!("Failed to parse and import dat file: {:?}, {}", file, e);
		}
	}

	populate_all_clone_of_ids(conn).await?;

	Ok(())
}
