use crate::providers::retroachievements::RetroAchievementsClient;
use std::sync::Arc;

pub mod game;
pub mod platform;

pub async fn match_db_to_retroachievements_entities(
	client: Arc<RetroAchievementsClient>,
	db_conn: &sea_orm::DbConn,
) -> anyhow::Result<()> {
	platform::match_platforms_to_retroachievements(client.clone(), db_conn).await?;
	game::match_games_to_retroachievements(client, db_conn).await
}
