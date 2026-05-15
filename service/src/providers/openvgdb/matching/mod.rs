use crate::providers::openvgdb::OpenVgdbClient;
use std::sync::Arc;

pub mod game;

pub async fn match_db_to_openvgdb_entities(
	client: Arc<OpenVgdbClient>,
	db_conn: &sea_orm::DbConn,
) -> anyhow::Result<()> {
	game::match_games_to_openvgdb(client, db_conn).await
}
