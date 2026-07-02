use crate::providers::screenscraper::ScreenScraperClient;
use log::warn;
use std::sync::Arc;

pub mod game;
pub mod platform;

pub async fn match_db_to_screenscraper_entities(
	client: Arc<ScreenScraperClient>,
	db_conn: &sea_orm::DbConn,
) -> anyhow::Result<()> {
	if client.is_quota_exhausted() {
		warn!("ScreenScraper quota exhausted before platform stage, skipping cycle");
		return Ok(());
	}
	platform::match_platforms_to_screenscraper(client.clone(), db_conn).await?;

	if client.is_quota_exhausted() {
		warn!("ScreenScraper quota exhausted before game stage, ending cycle early");
		return Ok(());
	}
	game::match_games_to_screenscraper(client, db_conn).await
}
