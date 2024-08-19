mod company;
mod game;
mod platform;

use crate::metadata::igdb::IgdbClient;
use crate::r#match::game::match_games_to_igdb;
use company::match_companies_to_igdb;
use lazy_static::lazy_static;
use log::info;
use platform::match_platforms_to_igdb;
use regex::Regex;
use sea_orm::DbConn;
use std::sync::Arc;

const PAGE_SIZE: u64 = 50;

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
	match_companies_to_igdb(igdb_client.clone(), db_conn).await?;
	info!("Finished matching companies to IGDB entities");

	match_platforms_to_igdb(igdb_client.clone(), db_conn).await?;
	info!("Finished matching platforms to IGDB entities");

	match_games_to_igdb(igdb_client.clone(), db_conn).await?;
	info!("Finished matching games to IGDB entities");
	Ok(())
}
