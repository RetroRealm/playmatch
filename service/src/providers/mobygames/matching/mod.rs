use crate::providers::mobygames::MobyGamesClient;
use std::sync::Arc;

pub mod game;
pub mod platform;

pub async fn match_db_to_mobygames_entities(
	client: Arc<MobyGamesClient>,
	db_conn: &sea_orm::DbConn,
) -> anyhow::Result<()> {
	platform::match_platforms_to_mobygames(client.clone(), db_conn).await?;
	game::match_games_to_mobygames(client, db_conn).await
}
