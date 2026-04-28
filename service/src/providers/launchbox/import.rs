use crate::db::launchbox::{
	bulk_insert_lb_alternate_names, bulk_insert_lb_images, bulk_upsert_lb_games,
	bulk_upsert_lb_platforms, latest_lb_import_md5, record_lb_import,
	truncate_lb_alternate_names_and_images,
};
use crate::fs::calculate_md5;
use crate::http::abstraction::RequestClientExt;
use crate::ingestion::archive::extract_if_archived;
use anyhow::{Context, anyhow, bail};
use entity::{
	launchbox_game, launchbox_game_alternate_name, launchbox_game_image, launchbox_platform,
};
use futures_util::StreamExt;
use log::{debug, info};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use reqwest::Client;
use sea_orm::ActiveValue::Set;
use sea_orm::DbConn;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::fs as tokio_fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

pub const DEFAULT_METADATA_URL: &str = "https://gamesdb.launchbox-app.com/Metadata.zip";
const LB_TMP_DIR: &str = "tmp/launchbox";
const LB_BATCH_SIZE: usize = 2000;
const LB_CHANNEL_DEPTH: usize = 8;

#[derive(Debug, Default, Clone)]
pub struct ImportOutcome {
	pub imported_md5: String,
	pub skipped_because_unchanged: bool,
	pub games: usize,
	pub platforms: usize,
	pub alternate_names: usize,
	pub images: usize,
	pub elapsed_ms: u128,
}

/// Download the LaunchBox metadata zip, extract it, and import the contents
/// into the launchbox_* tables. Idempotent: skips the entire pipeline when the
/// downloaded zip's MD5 matches the most recent recorded import.
pub async fn ensure_imported(
	http: &Client,
	db_conn: &DbConn,
	metadata_url: &str,
) -> anyhow::Result<ImportOutcome> {
	let started = Instant::now();
	info!("LaunchBox metadata import starting");

	let cwd = std::env::current_dir()?;
	let tmp_dir = cwd.join(LB_TMP_DIR);
	tokio_fs::create_dir_all(&tmp_dir).await?;

	let zip_path = tmp_dir.join("metadata.zip");
	download_zip(http, metadata_url, &zip_path).await?;

	let md5 = calculate_md5(&zip_path).await?;
	debug!("LaunchBox zip md5={md5}");

	if let Some(prev) = latest_lb_import_md5(db_conn).await?
		&& prev == md5
	{
		info!("LaunchBox import skipped (md5 unchanged: {md5})");
		cleanup_tmp(&tmp_dir).await;
		return Ok(ImportOutcome {
			imported_md5: md5,
			skipped_because_unchanged: true,
			elapsed_ms: started.elapsed().as_millis(),
			..Default::default()
		});
	}

	extract_if_archived(&zip_path).await?;
	let xml_path = locate_metadata_xml(&tmp_dir).await?;
	debug!("LaunchBox extracted XML at {xml_path:?}");

	truncate_lb_alternate_names_and_images(db_conn).await?;

	let counts = stream_parse_and_insert(&xml_path, db_conn).await?;

	record_lb_import(
		&md5,
		counts.games as i32,
		counts.platforms as i32,
		counts.alternate_names as i32,
		counts.images as i32,
		db_conn,
	)
	.await?;

	cleanup_tmp(&tmp_dir).await;

	let elapsed_ms = started.elapsed().as_millis();
	info!(
		"LaunchBox import done in {} ms: {} platforms, {} games, {} alt names, {} images (md5={})",
		elapsed_ms, counts.platforms, counts.games, counts.alternate_names, counts.images, md5,
	);

	Ok(ImportOutcome {
		imported_md5: md5,
		skipped_because_unchanged: false,
		games: counts.games,
		platforms: counts.platforms,
		alternate_names: counts.alternate_names,
		images: counts.images,
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

async fn locate_metadata_xml(tmp_dir: &Path) -> anyhow::Result<PathBuf> {
	let extracted_dir = tmp_dir.join("metadata");
	let mut entries = tokio_fs::read_dir(&extracted_dir).await?;
	while let Some(entry) = entries.next_entry().await? {
		let path = entry.path();
		if path
			.extension()
			.and_then(|e| e.to_str())
			.map(|e| e.eq_ignore_ascii_case("xml"))
			.unwrap_or(false)
			&& path
				.file_name()
				.and_then(|n| n.to_str())
				.map(|n| n.eq_ignore_ascii_case("metadata.xml"))
				.unwrap_or(false)
		{
			return Ok(path);
		}
	}
	bail!("Metadata.xml not found in extracted launchbox archive at {extracted_dir:?}")
}

async fn cleanup_tmp(tmp_dir: &Path) {
	if let Err(e) = tokio_fs::remove_dir_all(tmp_dir).await {
		debug!("LaunchBox tmp cleanup failed (non-fatal): {e}");
	}
}

#[derive(Debug, Default, Clone, Copy)]
struct Counts {
	platforms: usize,
	games: usize,
	alternate_names: usize,
	images: usize,
}

enum Batch {
	Platforms(Vec<launchbox_platform::ActiveModel>),
	Games(Vec<launchbox_game::ActiveModel>),
	AlternateNames(Vec<launchbox_game_alternate_name::ActiveModel>),
	Images(Vec<launchbox_game_image::ActiveModel>),
}

async fn stream_parse_and_insert(xml_path: &Path, db_conn: &DbConn) -> anyhow::Result<Counts> {
	let (tx, mut rx) = mpsc::channel::<Batch>(LB_CHANNEL_DEPTH);
	let xml_path_owned = xml_path.to_owned();

	let parse_handle = tokio::task::spawn_blocking(move || -> anyhow::Result<Counts> {
		parse_xml_file(&xml_path_owned, &tx)
	});

	let mut counts = Counts::default();
	while let Some(batch) = rx.recv().await {
		match batch {
			Batch::Platforms(rows) => {
				let n = rows.len();
				bulk_upsert_lb_platforms(rows, db_conn).await?;
				counts.platforms += n;
			}
			Batch::Games(rows) => {
				let n = rows.len();
				bulk_upsert_lb_games(rows, db_conn).await?;
				counts.games += n;
				if counts.games.is_multiple_of(50_000) {
					info!("LaunchBox: imported {} games", counts.games);
				}
			}
			Batch::AlternateNames(rows) => {
				let n = rows.len();
				bulk_insert_lb_alternate_names(rows, db_conn).await?;
				counts.alternate_names += n;
			}
			Batch::Images(rows) => {
				let n = rows.len();
				bulk_insert_lb_images(rows, db_conn).await?;
				counts.images += n;
			}
		}
	}

	parse_handle
		.await
		.context("LaunchBox XML parse task panicked")??;

	Ok(counts)
}

fn parse_xml_file(path: &Path, tx: &mpsc::Sender<Batch>) -> anyhow::Result<Counts> {
	let file = File::open(path)?;
	let mut reader = Reader::from_reader(BufReader::with_capacity(1024 * 1024, file));
	reader.config_mut().trim_text(true);
	let mut buf: Vec<u8> = Vec::with_capacity(8 * 1024);

	let mut platforms_buf: Vec<launchbox_platform::ActiveModel> = Vec::with_capacity(LB_BATCH_SIZE);
	let mut games_buf: Vec<launchbox_game::ActiveModel> = Vec::with_capacity(LB_BATCH_SIZE);
	let mut alts_buf: Vec<launchbox_game_alternate_name::ActiveModel> =
		Vec::with_capacity(LB_BATCH_SIZE);
	let mut images_buf: Vec<launchbox_game_image::ActiveModel> = Vec::with_capacity(LB_BATCH_SIZE);
	let mut counts = Counts::default();

	loop {
		match reader.read_event_into(&mut buf)? {
			Event::Start(start) => {
				let tag = start.name().as_ref().to_owned();
				match tag.as_slice() {
					b"Game" => {
						let fields = read_flat_element(&mut reader, b"Game")?;
						if let Some(am) = build_game_active_model(&fields) {
							games_buf.push(am);
							flush_if_full(&mut games_buf, tx, Batch::Games)?;
						}
					}
					b"Platform" => {
						let fields = read_flat_element(&mut reader, b"Platform")?;
						if let Some(am) = build_platform_active_model(&fields) {
							platforms_buf.push(am);
							flush_if_full(&mut platforms_buf, tx, Batch::Platforms)?;
						}
					}
					b"GameAlternateName" => {
						let fields = read_flat_element(&mut reader, b"GameAlternateName")?;
						if let Some(am) = build_alt_active_model(&fields) {
							alts_buf.push(am);
							flush_if_full(&mut alts_buf, tx, Batch::AlternateNames)?;
						}
					}
					b"GameImage" => {
						let fields = read_flat_element(&mut reader, b"GameImage")?;
						if let Some(am) = build_image_active_model(&fields) {
							images_buf.push(am);
							flush_if_full(&mut images_buf, tx, Batch::Images)?;
						}
					}
					_ => {}
				}
			}
			Event::Eof => break,
			_ => {}
		}
		buf.clear();
	}

	if !platforms_buf.is_empty() {
		counts.platforms += platforms_buf.len();
		send_batch(tx, Batch::Platforms(platforms_buf))?;
	}
	if !games_buf.is_empty() {
		counts.games += games_buf.len();
		send_batch(tx, Batch::Games(games_buf))?;
	}
	if !alts_buf.is_empty() {
		counts.alternate_names += alts_buf.len();
		send_batch(tx, Batch::AlternateNames(alts_buf))?;
	}
	if !images_buf.is_empty() {
		counts.images += images_buf.len();
		send_batch(tx, Batch::Images(images_buf))?;
	}

	Ok(counts)
}

fn flush_if_full<T, F>(buf: &mut Vec<T>, tx: &mpsc::Sender<Batch>, wrap: F) -> anyhow::Result<()>
where
	F: FnOnce(Vec<T>) -> Batch,
{
	if buf.len() >= LB_BATCH_SIZE {
		let drained = std::mem::replace(buf, Vec::with_capacity(LB_BATCH_SIZE));
		send_batch(tx, wrap(drained))?;
	}
	Ok(())
}

fn send_batch(tx: &mpsc::Sender<Batch>, batch: Batch) -> anyhow::Result<()> {
	tx.blocking_send(batch)
		.map_err(|e| anyhow!("LaunchBox import channel closed: {e}"))
}

fn read_flat_element<R: std::io::BufRead>(
	reader: &mut Reader<R>,
	end_tag: &[u8],
) -> anyhow::Result<HashMap<String, String>> {
	let mut fields = HashMap::new();
	let mut current_field: Option<String> = None;
	let mut current_value = String::new();
	let mut buf: Vec<u8> = Vec::with_capacity(1024);

	loop {
		match reader.read_event_into(&mut buf)? {
			Event::Start(e) => {
				let name = std::str::from_utf8(e.name().as_ref())?.to_string();
				current_field = Some(name);
				current_value.clear();
			}
			Event::Text(t) if current_field.is_some() => {
				let decoded = t.decode()?;
				let unescaped = quick_xml::escape::unescape(&decoded)?;
				current_value.push_str(&unescaped);
			}
			Event::CData(c) if current_field.is_some() => {
				current_value.push_str(std::str::from_utf8(c.into_inner().as_ref())?);
			}
			Event::Empty(e) => {
				let name = std::str::from_utf8(e.name().as_ref())?.to_string();
				fields.insert(name, String::new());
			}
			Event::End(e) => {
				if e.name().as_ref() == end_tag {
					if let Some(field) = current_field.take() {
						fields.insert(field, std::mem::take(&mut current_value));
					}
					return Ok(fields);
				}
				if let Some(field) = current_field.take() {
					fields.insert(field, std::mem::take(&mut current_value));
				}
			}
			Event::Eof => bail!(
				"unexpected EOF inside <{}>",
				String::from_utf8_lossy(end_tag)
			),
			_ => {}
		}
		buf.clear();
	}
}

fn opt(fields: &HashMap<String, String>, key: &str) -> Option<String> {
	fields
		.get(key)
		.map(|s| s.trim().to_string())
		.filter(|s| !s.is_empty())
}

fn opt_bool(fields: &HashMap<String, String>, key: &str) -> Option<bool> {
	opt(fields, key).and_then(|s| match s.to_ascii_lowercase().as_str() {
		"true" | "1" | "yes" => Some(true),
		"false" | "0" | "no" => Some(false),
		_ => None,
	})
}

fn opt_i32(fields: &HashMap<String, String>, key: &str) -> Option<i32> {
	opt(fields, key).and_then(|s| s.parse::<i32>().ok())
}

fn opt_i64(fields: &HashMap<String, String>, key: &str) -> Option<i64> {
	opt(fields, key).and_then(|s| s.parse::<i64>().ok())
}

fn opt_f32(fields: &HashMap<String, String>, key: &str) -> Option<f32> {
	opt(fields, key).and_then(|s| s.parse::<f32>().ok())
}

fn build_platform_active_model(
	fields: &HashMap<String, String>,
) -> Option<launchbox_platform::ActiveModel> {
	let name = opt(fields, "Name")?;
	Some(launchbox_platform::ActiveModel {
		name: Set(name),
		emulated: Set(opt_bool(fields, "Emulated")),
		release_date: Set(opt(fields, "ReleaseDate")),
		developer: Set(opt(fields, "Developer")),
		manufacturer: Set(opt(fields, "Manufacturer")),
		cpu: Set(opt(fields, "Cpu")),
		memory: Set(opt(fields, "Memory")),
		graphics: Set(opt(fields, "Graphics")),
		sound: Set(opt(fields, "Sound")),
		display: Set(opt(fields, "Display")),
		media: Set(opt(fields, "Media")),
		max_controllers: Set(opt(fields, "MaxControllers")),
		notes: Set(opt(fields, "Notes")),
		category: Set(opt(fields, "Category")),
		..Default::default()
	})
}

fn build_game_active_model(
	fields: &HashMap<String, String>,
) -> Option<launchbox_game::ActiveModel> {
	let database_id = opt_i64(fields, "DatabaseID")?;
	let name = opt(fields, "Name")?;
	let platform_name = opt(fields, "Platform")?;
	Some(launchbox_game::ActiveModel {
		database_id: Set(database_id),
		name: Set(name),
		platform_name: Set(platform_name),
		release_date: Set(opt(fields, "ReleaseDate")),
		release_year: Set(opt_i32(fields, "ReleaseYear")),
		overview: Set(opt(fields, "Overview")),
		developer: Set(opt(fields, "Developer")),
		publisher: Set(opt(fields, "Publisher")),
		genres: Set(opt(fields, "Genres")),
		max_players: Set(opt_i32(fields, "MaxPlayers")),
		cooperative: Set(opt_bool(fields, "Cooperative")),
		esrb: Set(opt(fields, "ESRB")),
		release_type: Set(opt(fields, "ReleaseType")),
		status: Set(opt(fields, "Status")),
		wikipedia_url: Set(opt(fields, "WikipediaURL")),
		video_url: Set(opt(fields, "VideoURL")),
		community_rating: Set(opt_f32(fields, "CommunityRating")),
		community_rating_count: Set(opt_i32(fields, "CommunityRatingCount")),
		..Default::default()
	})
}

fn build_alt_active_model(
	fields: &HashMap<String, String>,
) -> Option<launchbox_game_alternate_name::ActiveModel> {
	let database_id = opt_i64(fields, "DatabaseID")?;
	let name = opt(fields, "AlternateName")?;
	Some(launchbox_game_alternate_name::ActiveModel {
		launchbox_game_database_id: Set(database_id),
		name: Set(name),
		region: Set(opt(fields, "Region")),
		..Default::default()
	})
}

fn build_image_active_model(
	fields: &HashMap<String, String>,
) -> Option<launchbox_game_image::ActiveModel> {
	let database_id = opt_i64(fields, "DatabaseID")?;
	let file_name = opt(fields, "FileName")?;
	let image_type = opt(fields, "Type")?;
	Some(launchbox_game_image::ActiveModel {
		launchbox_game_database_id: Set(database_id),
		file_name: Set(file_name),
		image_type: Set(image_type),
		region: Set(opt(fields, "Region")),
		..Default::default()
	})
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::io::Cursor;

	fn parse_collect(
		xml: &str,
	) -> (
		Vec<launchbox_platform::ActiveModel>,
		Vec<launchbox_game::ActiveModel>,
		Vec<launchbox_game_alternate_name::ActiveModel>,
		Vec<launchbox_game_image::ActiveModel>,
	) {
		let mut reader = Reader::from_reader(Cursor::new(xml));
		reader.config_mut().trim_text(true);
		let mut buf = Vec::new();
		let mut platforms = Vec::new();
		let mut games = Vec::new();
		let mut alts = Vec::new();
		let mut images = Vec::new();
		loop {
			match reader.read_event_into(&mut buf).unwrap() {
				Event::Start(start) => {
					let tag = start.name().as_ref().to_owned();
					match tag.as_slice() {
						b"Game" => {
							let f = read_flat_element(&mut reader, b"Game").unwrap();
							if let Some(am) = build_game_active_model(&f) {
								games.push(am);
							}
						}
						b"Platform" => {
							let f = read_flat_element(&mut reader, b"Platform").unwrap();
							if let Some(am) = build_platform_active_model(&f) {
								platforms.push(am);
							}
						}
						b"GameAlternateName" => {
							let f = read_flat_element(&mut reader, b"GameAlternateName").unwrap();
							if let Some(am) = build_alt_active_model(&f) {
								alts.push(am);
							}
						}
						b"GameImage" => {
							let f = read_flat_element(&mut reader, b"GameImage").unwrap();
							if let Some(am) = build_image_active_model(&f) {
								images.push(am);
							}
						}
						_ => {}
					}
				}
				Event::Eof => break,
				_ => {}
			}
			buf.clear();
		}
		(platforms, games, alts, images)
	}

	#[test]
	fn parses_typical_launchbox_metadata_xml() {
		let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<LaunchBox>
  <Platform>
    <Name>Nintendo Entertainment System</Name>
    <Emulated>true</Emulated>
    <ReleaseDate>1985-10-18</ReleaseDate>
    <Developer>Nintendo</Developer>
    <Manufacturer>Nintendo</Manufacturer>
    <Category>Console</Category>
  </Platform>
  <Game>
    <Name>Super Mario Bros.</Name>
    <DatabaseID>1234</DatabaseID>
    <Platform>Nintendo Entertainment System</Platform>
    <ReleaseDate>1985-09-13</ReleaseDate>
    <ReleaseYear>1985</ReleaseYear>
    <Developer>Nintendo</Developer>
    <Publisher>Nintendo</Publisher>
    <Genres>Platform</Genres>
    <MaxPlayers>2</MaxPlayers>
    <Cooperative>false</Cooperative>
    <ESRB>Everyone</ESRB>
    <CommunityRating>4.5</CommunityRating>
    <CommunityRatingCount>1234</CommunityRatingCount>
  </Game>
  <Game>
    <Name>Tetris</Name>
    <DatabaseID>5678</DatabaseID>
    <Platform>Game Boy</Platform>
  </Game>
  <GameAlternateName>
    <DatabaseID>1234</DatabaseID>
    <AlternateName>スーパーマリオブラザーズ</AlternateName>
    <Region>Japan</Region>
  </GameAlternateName>
  <GameAlternateName>
    <DatabaseID>1234</DatabaseID>
    <AlternateName>Super Mario Bros</AlternateName>
  </GameAlternateName>
  <GameImage>
    <DatabaseID>1234</DatabaseID>
    <FileName>Nintendo Entertainment System/Super Mario Bros - Box - Front - 01.png</FileName>
    <Type>Box - Front</Type>
    <Region>North America</Region>
  </GameImage>
  <GameImage>
    <DatabaseID>1234</DatabaseID>
    <FileName>Nintendo Entertainment System/Super Mario Bros - Banner.png</FileName>
    <Type>Banner</Type>
  </GameImage>
  <GameImage>
    <DatabaseID>5678</DatabaseID>
    <FileName>Game Boy/Tetris - Screenshot - Gameplay - 01.png</FileName>
    <Type>Screenshot - Gameplay</Type>
  </GameImage>
</LaunchBox>"#;

		let (platforms, games, alts, images) = parse_collect(xml);
		assert_eq!(platforms.len(), 1);
		assert_eq!(games.len(), 2);
		assert_eq!(alts.len(), 2);
		assert_eq!(images.len(), 3);
	}

	#[test]
	fn drops_records_missing_required_fields() {
		let xml = r#"<LaunchBox>
  <Game><Name>Missing ID</Name><Platform>NES</Platform></Game>
  <Game><DatabaseID>1</DatabaseID><Platform>NES</Platform></Game>
  <GameAlternateName><AlternateName>orphan</AlternateName></GameAlternateName>
  <GameImage><DatabaseID>2</DatabaseID><Type>Banner</Type></GameImage>
</LaunchBox>"#;
		let (_, games, alts, images) = parse_collect(xml);
		assert_eq!(games.len(), 0);
		assert_eq!(alts.len(), 0);
		assert_eq!(images.len(), 0);
	}
}
