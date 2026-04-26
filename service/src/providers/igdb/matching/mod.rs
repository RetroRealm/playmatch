mod company;
mod game;
mod platform;

use self::game::match_games_to_igdb;
use crate::providers::igdb::IgdbClient;
use company::match_companies_to_igdb;
use log::info;
use platform::match_platforms_to_igdb;
use sea_orm::DbConn;
use std::sync::Arc;

pub async fn match_db_to_igdb_entities(
	igdb_client: Arc<IgdbClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	match_companies_to_igdb(igdb_client.clone(), db_conn).await?;
	info!("Finished matching companies to IGDB");

	match_platforms_to_igdb(igdb_client.clone(), db_conn).await?;
	info!("Finished matching platforms to IGDB");

	match_games_to_igdb(igdb_client.clone(), db_conn).await?;
	info!("Finished matching games to IGDB");
	Ok(())
}
