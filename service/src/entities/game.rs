use crate::db::game::{GAME_NAME_SEARCH_DEFAULT_LIMIT, search_games_by_name};
use crate::model::GameNameSearchResult;
use sea_orm::DbConn;
use sea_orm::prelude::Uuid;

pub async fn search_games_by_name_and_platform(
	query: &str,
	platform_id: Option<Uuid>,
	limit: Option<u64>,
	db_conn: &DbConn,
) -> anyhow::Result<Vec<GameNameSearchResult>> {
	let limit = limit.unwrap_or(GAME_NAME_SEARCH_DEFAULT_LIMIT);
	let rows = search_games_by_name(query, platform_id, limit, db_conn).await?;

	Ok(rows
		.into_iter()
		.map(
			|(id, name, platform_id, platform_name)| GameNameSearchResult {
				id,
				name,
				platform_id,
				platform_name,
			},
		)
		.collect())
}
