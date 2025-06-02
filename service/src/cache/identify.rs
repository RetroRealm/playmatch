use crate::game::identify_game;
use crate::model::{GameFileMatchSearch, GameMatchResult};
use cached::proc_macro::cached;
use cached::TimedSizedCache;
use sea_orm::DbConn;

const CACHE_SIZE: usize = 100_000;
const CACHE_LIFESPAN: u64 = 86400;
const REFRESH_ON_RETRIEVE: bool = true;

#[cached(
	result = true,
	ty = "TimedSizedCache<GameFileMatchSearch, GameMatchResult>",
	create = "{ TimedSizedCache::with_size_and_lifespan_and_refresh(CACHE_SIZE, CACHE_LIFESPAN, REFRESH_ON_RETRIEVE) }",
	convert = r#"{ query.clone() }"#
)]
pub async fn identify_game_cached(
	db_conn: &DbConn,
	query: GameFileMatchSearch,
) -> anyhow::Result<GameMatchResult> {
	identify_game(query, db_conn).await
}
