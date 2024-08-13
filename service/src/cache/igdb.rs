use crate::metadata::igdb::model::{AgeRating, AlternativeName, Game};
use crate::metadata::igdb::IgdbClient;
use cached::proc_macro::cached;
use cached::TimedCache;

#[cached(
	result = true,
	ty = "TimedCache<i64, Option<Game>>",
	create = "{ TimedCache::with_lifespan(86400) }",
	convert = r#"{ id.clone() }"#
)]
pub async fn get_game_by_id_cached(client: &IgdbClient, id: i64) -> anyhow::Result<Option<Game>> {
	client.get_game_by_id(id).await
}

#[cached(
	result = true,
	ty = "TimedCache<Vec<i64>, Vec<Game>>",
	create = "{ TimedCache::with_lifespan(86400) }",
	convert = r#"{ ids.clone() }"#
)]
pub async fn get_games_by_ids_cached(
	client: &IgdbClient,
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
	client: &IgdbClient,
	query: String,
) -> anyhow::Result<Vec<Game>> {
	client.search_game_by_name(query).await
}

#[cached(
	result = true,
	ty = "TimedCache<i64, Option<AgeRating>>",
	create = "{ TimedCache::with_lifespan(86400) }",
	convert = r#"{ id.clone() }"#
)]
pub async fn get_age_rating_by_id_cached(
	client: &IgdbClient,
	id: i64,
) -> anyhow::Result<Option<AgeRating>> {
	client.get_age_rating_by_id(id).await
}

#[cached(
	result = true,
	ty = "TimedCache<Vec<i64>, Vec<AgeRating>>",
	create = "{ TimedCache::with_lifespan(86400) }",
	convert = r#"{ ids.clone() }"#
)]
pub async fn get_age_ratings_by_id_cached(
	client: &IgdbClient,
	ids: Vec<i64>,
) -> anyhow::Result<Vec<AgeRating>> {
	client.get_age_ratings_by_id(ids).await
}

#[cached(
	result = true,
	ty = "TimedCache<i64, Option<AlternativeName>>",
	create = "{ TimedCache::with_lifespan(86400) }",
	convert = r#"{ id.clone() }"#
)]
pub async fn get_alternative_name_by_id_cached(
	client: &IgdbClient,
	id: i64,
) -> anyhow::Result<Option<AlternativeName>> {
	client.get_alternative_name_by_id(id).await
}

#[cached(
	result = true,
	ty = "TimedCache<Vec<i64>, Vec<AlternativeName>>",
	create = "{ TimedCache::with_lifespan(86400) }",
	convert = r#"{ ids.clone() }"#
)]
pub async fn get_alternative_names_by_id_cached(
	client: &IgdbClient,
	ids: Vec<i64>,
) -> anyhow::Result<Vec<AlternativeName>> {
	client.get_alternative_names_by_id(ids).await
}
