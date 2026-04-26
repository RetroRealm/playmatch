use crate::providers::steamgriddb::SteamGridDbClient;
use std::sync::Arc;

pub mod game;

pub async fn match_steamgriddb_to_db_games(
	client: Arc<SteamGridDbClient>,
	db_conn: &sea_orm::DbConn,
) -> anyhow::Result<()> {
	game::match_games_to_steamgriddb(client, db_conn).await
}
