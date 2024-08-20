use crate::dat::shared::model::{Datafile, Game};
use crate::dat::shared::regex::{DAT_NUMBER_REGEX, DAT_TAG_REGEX};
use crate::db::company::create_or_find_company_by_name;
use crate::db::dat_file::{create_or_update_dat_file, DatFileCreateOrUpdateInput};
use crate::db::dat_file_import::create_dat_file_import;
use crate::db::game::{find_game_by_name_and_dat_file_id, insert_game};
use crate::db::game_file::insert_game_file_bulk;
use crate::db::platform::create_or_find_platform_by_name;
use entity::{company, dat_file_import, platform};
use log::info;
use sea_orm::prelude::Uuid;
use sea_orm::DbConn;
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
			.chunks(64)
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

					if result.is_some() {
						return Ok(());
					}

					let roms = game.rom.clone();

					let game_release = insert_game(import.id, game, &conn).await?;

					// When we insert too many sqlx-postgres panics, so we chunk the inserts
					for chunk in roms.chunks(25) {
						insert_game_file_bulk(chunk.to_vec(), game_release.id, &conn).await?;
					}

					Ok(())
				}));
			}

			for future in futures {
				future.await??;
			}
		}

		info!("Imported DAT file: {}", path.display());
	}

	Ok(())
}

fn parse_company_and_platform(
	dat: &Datafile,
) -> anyhow::Result<(Option<String>, String, Vec<String>)> {
	let split = dat.header.name.split(" - ").collect::<Vec<&str>>();

	if split.is_empty() {
		return Err(anyhow::anyhow!("No company or system found"));
	}

	let subset = &dat.header.subset;
	let version = &dat.header.version;
	let mut tags = Vec::new();
	let mut company = String::new();
	let mut platform_parts = Vec::new();

	let mut real_index = 0;
	for part in split {
		if let Some(subset) = subset {
			if subset == part {
				continue;
			}
		}

		if real_index == 0 {
			company = part.to_string();
		} else {
			platform_parts.push(part.to_string());
		}

		real_index += 1;
	}

	let mut platform = platform_parts.join(" - ");

	if platform.is_empty() {
		platform = company.clone();
		company = "".to_string();
	}

	// replace version out of name as that's not needed for tags
	platform = platform.replace(format!(" ({})", version).as_str(), "");

	for tag in DAT_TAG_REGEX.captures_iter(&platform.clone()) {
		let tag = tag.get(1).map(|x| x.as_str()).unwrap_or_default();
		tags.push(tag.to_owned());
		platform = platform.replace(&format!(" ({})", tag), "");
	}

	Ok((
		if company.is_empty() {
			None
		} else {
			Some(company)
		},
		platform,
		tags,
	))
}

pub fn sanitize_dat_string(mut file_name: String, file_extension: &str, version: &str) -> String {
	file_name = file_name.replace(format!(" ({})", version).as_str(), "");

	for tag in DAT_NUMBER_REGEX.captures_iter(&file_name.clone()) {
		let tag = tag.get(0).map(|x| x.as_str()).unwrap_or_default();
		file_name = file_name.replace(&format!(" {}", tag), "");
	}

	file_name = file_name.replace(format!(".{}", file_extension).as_str(), "");

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
