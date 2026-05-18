use crate::providers::thegamesdb::TheGamesDbClient;
use std::sync::Arc;

pub mod game;

pub async fn match_db_to_thegamesdb_entities(
	client: Arc<TheGamesDbClient>,
	db_conn: &sea_orm::DbConn,
) -> anyhow::Result<()> {
	game::match_games_to_thegamesdb(client, db_conn).await
}
