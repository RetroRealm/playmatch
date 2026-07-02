use crate::db::openvgdb::{
	bulk_upsert_openvgdb_releases, bulk_upsert_openvgdb_roms, latest_openvgdb_import_md5,
	record_openvgdb_import,
};
use crate::fs::calculate_md5;
use crate::http::abstraction::RequestClientExt;
use crate::ingestion::archive::extract_if_archived;
use anyhow::{Context, bail};
use entity::{openvgdb_release, openvgdb_rom};
use futures_util::StreamExt;
use log::{debug, info};
use reqwest::Client;
use sea_orm::ActiveValue::Set;
use sea_orm::DbConn;
use sqlx::Row;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::fs as tokio_fs;
use tokio::io::AsyncWriteExt;

pub const DEFAULT_METADATA_URL: &str =
	"https://github.com/OpenVGDB/OpenVGDB/releases/latest/download/openvgdb.zip";
const OVGDB_TMP_DIR: &str = "tmp/openvgdb";
const OVGDB_BATCH_SIZE: usize = 2000;

#[derive(Debug, Default, Clone)]
pub struct ImportOutcome {
	pub imported_md5: String,
	pub skipped_because_unchanged: bool,
	pub roms: usize,
	pub releases: usize,
	pub elapsed_ms: u128,
}

/// Download the OpenVGDB zip, extract the bundled SQLite database, and import
/// its ROMs and RELEASES into the openvgdb_* tables. Idempotent: skips the
/// pipeline when the downloaded zip's MD5 matches the most recent recorded
/// import.
pub async fn ensure_imported(
	http: &Client,
	db_conn: &DbConn,
	metadata_url: &str,
) -> anyhow::Result<ImportOutcome> {
	let started = Instant::now();
	info!("OpenVGDB metadata import starting");

	let cwd = std::env::current_dir()?;
	let tmp_dir = cwd.join(OVGDB_TMP_DIR);
	tokio_fs::create_dir_all(&tmp_dir).await?;

	let zip_path = tmp_dir.join("openvgdb.zip");
	download_zip(http, metadata_url, &zip_path).await?;

	let md5 = calculate_md5(&zip_path).await?;
	debug!("OpenVGDB zip md5={md5}");

	if let Some(prev) = latest_openvgdb_import_md5(db_conn).await?
		&& prev == md5
	{
		info!("OpenVGDB import skipped (md5 unchanged: {md5})");
		crate::metrics::record_openvgdb_import_records("zip", "skipped_unchanged", 1);
		cleanup_tmp(&tmp_dir).await;
		return Ok(ImportOutcome {
			imported_md5: md5,
			skipped_because_unchanged: true,
			elapsed_ms: started.elapsed().as_millis(),
			..Default::default()
		});
	}

	extract_if_archived(&zip_path).await?;
	let sqlite_path = locate_sqlite_file(&tmp_dir).await?;
	debug!("OpenVGDB extracted SQLite at {sqlite_path:?}");

	let counts = read_sqlite_and_insert(&sqlite_path, db_conn).await?;

	record_openvgdb_import(&md5, counts.roms as i32, counts.releases as i32, db_conn).await?;

	cleanup_tmp(&tmp_dir).await;

	let elapsed_ms = started.elapsed().as_millis();
	info!(
		"OpenVGDB import done in {} ms: {} ROMs, {} releases (md5={})",
		elapsed_ms, counts.roms, counts.releases, md5,
	);

	Ok(ImportOutcome {
		imported_md5: md5,
		skipped_because_unchanged: false,
		roms: counts.roms,
		releases: counts.releases,
		elapsed_ms,
	})
}

async fn download_zip(http: &Client, url: &str, dest: &Path) -> anyhow::Result<()> {
	let response = http
		.get_default_user_agent(url)
		.send()
		.await?
		.error_for_status()?;
	let mut file = tokio_fs::File::create(dest).await?;
	let mut stream = response.bytes_stream();
	while let Some(chunk) = stream.next().await {
		let bytes = chunk?;
		file.write_all(&bytes).await?;
	}
	file.flush().await?;
	Ok(())
}

async fn locate_sqlite_file(tmp_dir: &Path) -> anyhow::Result<PathBuf> {
	let extracted_dir = tmp_dir.join("openvgdb");
	let mut entries = tokio_fs::read_dir(&extracted_dir).await?;
	while let Some(entry) = entries.next_entry().await? {
		let path = entry.path();
		if path
			.extension()
			.and_then(|e| e.to_str())
			.map(|e| e.eq_ignore_ascii_case("sqlite"))
			.unwrap_or(false)
		{
			return Ok(path);
		}
	}
	bail!("no .sqlite file found in extracted openvgdb archive at {extracted_dir:?}")
}

async fn cleanup_tmp(tmp_dir: &Path) {
	if let Err(e) = tokio_fs::remove_dir_all(tmp_dir).await {
		debug!("OpenVGDB tmp cleanup failed (non-fatal): {e}");
	}
}

#[derive(Debug, Default, Clone, Copy)]
struct Counts {
	roms: usize,
	releases: usize,
}

async fn read_sqlite_and_insert(sqlite_path: &Path, db_conn: &DbConn) -> anyhow::Result<Counts> {
	let opts = SqliteConnectOptions::new()
		.filename(sqlite_path)
		.read_only(true);
	let pool = SqlitePoolOptions::new()
		.max_connections(1)
		.connect_with(opts)
		.await
		.context("failed to open OpenVGDB SQLite database")?;

	let mut counts = Counts::default();

	let rom_rows = sqlx::query(
		"SELECT romID, systemID, regionID, romHashCRC, romHashMD5, romHashSHA1, \
		 romSize, romFileName, romExtensionlessFileName, romSerial FROM ROMs",
	)
	.fetch_all(&pool)
	.await
	.context("failed to read OpenVGDB ROMs table")?;

	let mut rom_batch: Vec<openvgdb_rom::ActiveModel> = Vec::with_capacity(OVGDB_BATCH_SIZE);
	for row in rom_rows {
		rom_batch.push(openvgdb_rom::ActiveModel {
			rom_id: Set(row.try_get::<i64, _>("romID")?),
			system_id: Set(row.try_get::<Option<i64>, _>("systemID")?.map(|v| v as i32)),
			region_id: Set(row.try_get::<Option<i64>, _>("regionID")?.map(|v| v as i32)),
			rom_hash_crc: Set(row.try_get::<Option<String>, _>("romHashCRC")?),
			rom_hash_md5: Set(row.try_get::<Option<String>, _>("romHashMD5")?),
			rom_hash_sha1: Set(row.try_get::<Option<String>, _>("romHashSHA1")?),
			rom_size: Set(row.try_get::<Option<i64>, _>("romSize")?),
			rom_file_name: Set(row.try_get::<Option<String>, _>("romFileName")?),
			rom_extensionless_file_name: Set(
				row.try_get::<Option<String>, _>("romExtensionlessFileName")?
			),
			rom_serial: Set(row.try_get::<Option<String>, _>("romSerial")?),
			..Default::default()
		});
		if rom_batch.len() >= OVGDB_BATCH_SIZE {
			let n = rom_batch.len();
			bulk_upsert_openvgdb_roms(std::mem::take(&mut rom_batch), db_conn).await?;
			counts.roms += n;
			crate::metrics::record_openvgdb_import_records("rom", "imported", n as u64);
			if counts.roms.is_multiple_of(50_000) {
				info!("OpenVGDB: imported {} ROMs", counts.roms);
			}
		}
	}
	if !rom_batch.is_empty() {
		let n = rom_batch.len();
		bulk_upsert_openvgdb_roms(rom_batch, db_conn).await?;
		counts.roms += n;
		crate::metrics::record_openvgdb_import_records("rom", "imported", n as u64);
	}

	let release_rows = sqlx::query(
		"SELECT releaseID, romID, releaseTitleName, TEMPregionLocalizedName, TEMPsystemName, \
		 releaseCoverFront, releaseCoverBack, releaseDescription, releaseDeveloper, \
		 releasePublisher, releaseGenre, releaseDate, releaseReferenceURL FROM RELEASES",
	)
	.fetch_all(&pool)
	.await
	.context("failed to read OpenVGDB RELEASES table")?;

	let mut release_batch: Vec<openvgdb_release::ActiveModel> =
		Vec::with_capacity(OVGDB_BATCH_SIZE);
	for row in release_rows {
		let Some(title_name) = row.try_get::<Option<String>, _>("releaseTitleName")? else {
			continue;
		};
		let release_date = row.try_get::<Option<String>, _>("releaseDate")?;
		let release_year = release_date.as_deref().and_then(parse_year);
		let title_name_normalized = Some(crate::matching::util::normalize_title(
			&title_name.to_lowercase(),
		));
		release_batch.push(openvgdb_release::ActiveModel {
			release_id: Set(row.try_get::<i64, _>("releaseID")?),
			rom_id: Set(row.try_get::<i64, _>("romID")?),
			title_name: Set(title_name),
			title_name_normalized: Set(title_name_normalized),
			region_name: Set(row.try_get::<Option<String>, _>("TEMPregionLocalizedName")?),
			system_name: Set(row.try_get::<Option<String>, _>("TEMPsystemName")?),
			cover_front: Set(row.try_get::<Option<String>, _>("releaseCoverFront")?),
			cover_back: Set(row.try_get::<Option<String>, _>("releaseCoverBack")?),
			description: Set(row.try_get::<Option<String>, _>("releaseDescription")?),
			developer: Set(row.try_get::<Option<String>, _>("releaseDeveloper")?),
			publisher: Set(row.try_get::<Option<String>, _>("releasePublisher")?),
			genre: Set(row.try_get::<Option<String>, _>("releaseGenre")?),
			release_date: Set(release_date),
			release_year: Set(release_year),
			reference_url: Set(row.try_get::<Option<String>, _>("releaseReferenceURL")?),
			..Default::default()
		});
		if release_batch.len() >= OVGDB_BATCH_SIZE {
			let n = release_batch.len();
			bulk_upsert_openvgdb_releases(std::mem::take(&mut release_batch), db_conn).await?;
			counts.releases += n;
			crate::metrics::record_openvgdb_import_records("release", "imported", n as u64);
		}
	}
	if !release_batch.is_empty() {
		let n = release_batch.len();
		bulk_upsert_openvgdb_releases(release_batch, db_conn).await?;
		counts.releases += n;
		crate::metrics::record_openvgdb_import_records("release", "imported", n as u64);
	}

	pool.close().await;
	Ok(counts)
}

/// OpenVGDB `releaseDate` values are inconsistent ("1996", "Sep 09, 1996",
/// "1996-09-09"), so pull the first plausible four-digit year out of the string.
fn parse_year(raw: &str) -> Option<i32> {
	let bytes = raw.as_bytes();
	for window in bytes.windows(4) {
		if window.iter().all(u8::is_ascii_digit) {
			let year: i32 = std::str::from_utf8(window).ok()?.parse().ok()?;
			if (1950..=2100).contains(&year) {
				return Some(year);
			}
		}
	}
	None
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parse_year_handles_common_formats() {
		assert_eq!(parse_year("1996"), Some(1996));
		assert_eq!(parse_year("Sep 09, 1996"), Some(1996));
		assert_eq!(parse_year("1996-09-09"), Some(1996));
		assert_eq!(parse_year("2003"), Some(2003));
	}

	#[test]
	fn parse_year_rejects_garbage() {
		assert_eq!(parse_year(""), None);
		assert_eq!(parse_year("unknown"), None);
		assert_eq!(parse_year("12"), None);
		assert_eq!(parse_year("1820"), None);
	}
}
