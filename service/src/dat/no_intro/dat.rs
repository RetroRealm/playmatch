use std::collections::VecDeque;
use std::path::Path;
use std::sync::Arc;

use chrono::Utc;
use log::debug;
use sea_orm::DbConn;
use tokio::{fs, task};
use tokio::task::JoinHandle;

use crate::dat::no_intro::model::{Datafile, Game};
use crate::db::query::find_game_release_by_name_and_platform_and_platform_company;

pub async fn read_and_import_no_intro_dat_files(path: &Path, conn: &DbConn) -> anyhow::Result<()> {
	let mut entries = fs::read_dir(path).await?;

	while let Some(entry) = entries.next_entry().await? {
		let path = entry.path();

		if path.is_file() && path.extension().unwrap_or_default() == "dat" {
			parse_and_import_no_intro_dat(&path, conn).await?;
		}
	}

	Ok(())
}

pub async fn parse_and_import_no_intro_dat(path: &Path, conn: &DbConn) -> anyhow::Result<()> {
	let dat = parse_no_intro_dat(path).await?;

	let mut split = dat.header.name.split(" - ").collect::<VecDeque<&str>>();

	if split.len() == 0 {
		return Err(anyhow::anyhow!("No company or system found"));
	}

	let company = split.pop_front().map(|c| c.to_string()).unwrap();
	let system = split.into_iter().collect::<Vec<&str>>().join(" - ");

	debug!("Parsed company: {:?}", &company);
	debug!("Parsed system: {:?}", &system);

	let now = Utc::now();

	let company_arc = Arc::new(company);
	let system_arc = Arc::new(system);

	if let Some(games) = dat.game {
		let games_chunked = games.chunks(64).map(|x: &[Game]| x.to_vec()).collect::<Vec<Vec<Game>>>();

		for game_chunk in games_chunked {
			let mut futures: Vec<JoinHandle<anyhow::Result<()>>> = vec![];

			for game in game_chunk {
				let company = company_arc.clone();
				let system = system_arc.clone();
				let conn = conn.clone();
				futures.push(task::spawn(async move {
					let conn = conn.clone();
					let company = company.clone();
					let system = system.clone();
					let result = find_game_release_by_name_and_platform_and_platform_company(
						&game.name, &system, &company, &conn,
					)
						.await?;

					if let Some(game_release) = result {
						debug!("Game release already exists: {:?}", game_release);
					} else {
						debug!("Game release does not exist {:?}, creating it", &game.name);
					}

					Ok::<>(())
				}));
			}

			for future in futures {
				future.await??;
			}
		}
	}

	Ok(())
}

pub async fn parse_no_intro_dat(path: &Path) -> anyhow::Result<Datafile> {
	let dat = fs::read_to_string(path).await?;

	let result: Datafile = serde_xml_rs::from_str(&dat)?;

	Ok(result)
}
