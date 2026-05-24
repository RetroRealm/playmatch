use crate::providers::hasheous::HasheousClient;
use std::sync::Arc;

pub mod game;

pub async fn match_db_to_hasheous_entities(
	client: Arc<HasheousClient>,
	db_conn: &sea_orm::DbConn,
) -> anyhow::Result<()> {
	game::match_games_to_hasheous(client, db_conn).await
}
