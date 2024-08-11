use crate::metadata::igdb::model::Game;
use crate::metadata::igdb::IgdbClient;
use cached::proc_macro::cached;
use cached::TimedCache;

#[cached(
	result = true,
	ty = "TimedCache<i64, Option<Game>>",
	create = "{ TimedCache::with_lifespan(86400) }",
	convert = r#"{ id.clone() }"#
)]
pub async fn get_game_by_id_cached(
	client: &mut IgdbClient,
	id: i64,
) -> anyhow::Result<Option<Game>> {
	client.get_game_by_id(id).await
}

#[cached(
	result = true,
	ty = "TimedCache<Vec<i64>, Vec<Game>>",
	create = "{ TimedCache::with_lifespan(86400) }",
	convert = r#"{ ids.clone() }"#
)]
pub async fn get_games_by_ids_cached(
	client: &mut IgdbClient,
	ids: Vec<i64>,
) -> anyhow::Result<Vec<Game>> {
	client.get_games_by_id(ids).await
}

#[cached(
	result = true,
	ty = "TimedCache<String, Vec<Game>>",
	create = "{ TimedCache::with_lifespan(86400) }",
	convert = r#"{ query.clone() }"#
)]
pub async fn search_game_by_name_cached(
	client: &mut IgdbClient,
	query: String,
) -> anyhow::Result<Vec<Game>> {
	client.search_game_by_name(query).await
}
