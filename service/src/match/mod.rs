mod company;
mod platform;

use crate::metadata::igdb::IgdbClient;
use company::match_companies_to_igdb;
use log::info;
use platform::match_platforms_to_igdb;
use sea_orm::DbConn;
use std::sync::Arc;

const PAGE_SIZE: u64 = 50;

pub async fn match_db_to_igdb_entities(
	igdb_client: Arc<IgdbClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	match_companies_to_igdb(igdb_client.clone(), db_conn).await?;
	info!("Finished matching companies to IGDB entities");

	match_platforms_to_igdb(igdb_client.clone(), db_conn).await?;
	info!("Finished matching platforms to IGDB entities");

	Ok(())
}
