mod company;
mod game;
mod platform;

use crate::metadata::igdb::IgdbClient;
use crate::r#match::game::match_games_to_igdb;
use company::match_companies_to_igdb;
use lazy_static::lazy_static;
use log::{error, info};
use platform::match_platforms_to_igdb;
use regex::Regex;
use sea_orm::{DbConn, FromQueryResult, Paginator, SelectModel};
use std::sync::Arc;
use tokio::task::JoinHandle;

const PAGE_SIZE: u64 = 100;

lazy_static! {
	static ref BRACKET_REGEX: Regex = Regex::new(r"\s*\(.*?\)").unwrap();
}

fn clean_name(input: &str) -> String {
	BRACKET_REGEX.replace_all(input, "").to_string()
}

pub async fn match_db_to_igdb_entities(
	igdb_client: Arc<IgdbClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	// We have to run it twice because of a bug in SeaORM with pagination on left joins which somehow misses the last page
	for _ in 0..2 {
		match_companies_to_igdb(igdb_client.clone(), db_conn).await?;
	}
	info!("Finished matching companies to IGDB");

	// same as above
	for _ in 0..2 {
		match_platforms_to_igdb(igdb_client.clone(), db_conn).await?;
	}
	info!("Finished matching platforms to IGDB");

	match_games_to_igdb(igdb_client.clone(), db_conn).await?;
	info!("Finished matching games to IGDB");
	Ok(())
}

async fn handle_db_pagination_chunked<T: FromQueryResult + Clone>(
	mut paginator: Paginator<'_, DbConn, SelectModel<T>>,
	igdb_client: Arc<IgdbClient>,
	db_conn: DbConn,
	f: impl Fn(T, Arc<IgdbClient>, DbConn) -> JoinHandle<anyhow::Result<()>>,
) -> anyhow::Result<()> {
	while let Some(inner_page) = paginator.fetch_and_next().await? {
		for inner_chunk in inner_page.chunks(4) {
			let mut results = vec![];

			for inner in inner_chunk.iter().cloned() {
				let igdb_client = igdb_client.clone();
				let db_conn = db_conn.clone();
				results.push(f(inner, igdb_client.clone(), db_conn));
			}

			for result in results {
				if let Err(e) = result.await? {
					error!("Error while matching to IGDB: {:?}", e);
				}
			}
		}
	}

	Ok(())
}
