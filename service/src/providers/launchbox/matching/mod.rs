use crate::providers::launchbox::LaunchBoxClient;
use std::sync::Arc;

pub mod game;
pub mod platform;

pub async fn match_db_to_launchbox_entities(
	client: Arc<LaunchBoxClient>,
	db_conn: &sea_orm::DbConn,
) -> anyhow::Result<()> {
	platform::match_platforms_to_launchbox(client.clone(), db_conn).await?;
	game::match_games_to_launchbox(client, db_conn).await
}
