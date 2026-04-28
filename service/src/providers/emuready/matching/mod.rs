use crate::providers::emuready::EmuReadyClient;
use std::sync::Arc;

pub mod game;
pub mod platform;

pub async fn match_db_to_emuready_entities(
	client: Arc<EmuReadyClient>,
	db_conn: &sea_orm::DbConn,
) -> anyhow::Result<()> {
	platform::match_platforms_to_emuready(client.clone(), db_conn).await?;
	game::match_games_to_emuready(client, db_conn).await
}
