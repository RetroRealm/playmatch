use crate::config::PARALLELISM;
use crate::db::dat_file_import::is_dat_already_in_history;
use crate::db::signature_group::find_signature_group_by_name;
use crate::fs;
use crate::fs::calculate_md5;
use crate::ingestion::parser::import::parse_and_import_dat_file;
use crate::ingestion::sources::{
	RedumpType, download_dats_site_legacy_dats, download_no_intro_dats, download_redump_dats,
};
use crate::matching::clone::populate_all_clone_of_ids;
use crate::matching::content_anchor::assign_all_content_anchors;
use anyhow::anyhow;
use fs::read_files_recursive;
use log::{debug, error, info};
use redis::aio::MultiplexedConnection;
use reqwest::Client;
use sea_orm::DbConn;
use std::path::PathBuf;

pub mod archive;
pub mod download;
pub mod parser;
pub mod sources;

pub(crate) const DATS_PATH: &str = "dats";
pub(crate) const TMP_PATH: &str = "tmp";

/// Download every DAT source, parse new files into the DB, and refresh clone-of relationships.
pub async fn download_and_parse_dats(
	client: &Client,
	conn: &DbConn,
	redis_conn: &mut MultiplexedConnection,
	force_import: bool,
) -> anyhow::Result<()> {
	let current_dir = std::env::current_dir()?;
	let tmp_dir = current_dir.join(DATS_PATH).join(TMP_PATH);
	tokio::fs::create_dir_all(&tmp_dir).await?;

	info!("Starting to download No-Intro DATs.");
	download_no_intro_dats(client).await?;
	info!("Successfully downloaded No-Intro DATs");

	info!("Starting to download Public Redump DATs.");
	download_redump_dats(client, RedumpType::Public).await?;
	info!("Successfully downloaded Public Redump DATs");
	info!("Starting to download Private Redump DATs.");
	download_redump_dats(client, RedumpType::Private).await?;
	info!("Successfully downloaded Private Redump DATs");

	info!("Starting to download dats.site Legacy DATs.");
	download_dats_site_legacy_dats(client).await?;
	info!("Successfully downloaded dats.site Legacy DATs");

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

				debug!("Calculated MD5 hash for file: {file:?}");

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
				"Skipping file: {file_name:?}, either has no .dat file extension or contains BIOS"
			);
			crate::metrics::record_dat_ingestion_file("unknown", "skipped_non_dat");
			continue;
		}

		let path_canonical = file.canonicalize()?;
		let parent = path_canonical.to_str().unwrap_or_default();

		let (signature_group, source_label) = if parent.contains("no-intro") {
			(Some("No-Intro"), "no_intro")
		} else if parent.contains("redump") {
			(Some("Redump"), "redump")
		} else if parent.contains("tosec") {
			(Some("TOSEC"), "tosec")
		} else if parent.contains("mame") {
			(Some("MAME"), "mame")
		} else if parent.contains("dats-site") {
			(Some("DatsSite-Legacy"), "dats_site_legacy")
		} else {
			(None, "unknown")
		};

		let already_imported = is_dat_already_in_history(&hash, conn).await?;

		if already_imported && !force_import {
			debug!("Dat file already imported: {file:?}");
			crate::metrics::record_dat_ingestion_file(source_label, "skipped_duplicate");
			continue;
		}

		let signature_group_entity = match signature_group {
			Some(signature_group_name) => find_signature_group_by_name(signature_group_name, conn)
				.await?
				.ok_or_else(|| {
					anyhow!("Signature Group not found in database (are all migrations applied?)")
				})?,
			None => return Err(anyhow!("Signature Group not found")),
		};

		debug!("Importing DAT file: {file:?}");
		match parse_and_import_dat_file(&file, signature_group_entity.id, &hash, conn, redis_conn)
			.await
		{
			Ok(_) => {
				info!("Imported DAT file: {}", file.display());
				crate::metrics::record_dat_ingestion_file(source_label, "imported");
			}
			Err(e) => {
				error!("Failed to parse and import dat file: {file:?}, {e}");
				crate::metrics::record_dat_ingestion_file(source_label, "parse_error");
			}
		}
	}
	info!("Finished importing all DAT files");

	populate_all_clone_of_ids(conn).await?;
	info!("Finished populating all clone_of relationships");

	assign_all_content_anchors(conn).await?;
	info!("Finished backfilling content anchors");

	Ok(())
}
