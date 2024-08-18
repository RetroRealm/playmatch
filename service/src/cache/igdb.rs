use crate::metadata::igdb::model::{
	AgeRating, AlternativeName, Artwork, Collection, Cover, ExternalGame, Franchise, Game, Genre,
};
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
	ty = "TimedCache<String, Vec<Game>>",
	create = "{ TimedCache::with_lifespan(86400) }",
	convert = r#"{ query.clone() }"#
)]
pub async fn search_game_by_name_cached(
	client: &IgdbClient,
	query: String,
) -> anyhow::Result<Vec<Game>> {
	client.search_game_by_name(&query).await
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
	ty = "TimedCache<i64, Option<Artwork>>",
	create = "{ TimedCache::with_lifespan(86400) }",
	convert = r#"{ id.clone() }"#
)]
pub async fn get_artwork_by_id_cached(
	client: &IgdbClient,
	id: i64,
) -> anyhow::Result<Option<Artwork>> {
	client.get_artwork_by_id(id).await
}

#[cached(
	result = true,
	ty = "TimedCache<i64, Option<Collection>>",
	create = "{ TimedCache::with_lifespan(86400) }",
	convert = r#"{ id.clone() }"#
)]
pub async fn get_collection_by_id_cached(
	client: &IgdbClient,
	id: i64,
) -> anyhow::Result<Option<Collection>> {
	client.get_collection_by_id(id).await
}

#[cached(
	result = true,
	ty = "TimedCache<i64, Option<Cover>>",
	create = "{ TimedCache::with_lifespan(86400) }",
	convert = r#"{ id.clone() }"#
)]
pub async fn get_cover_by_id_cached(client: &IgdbClient, id: i64) -> anyhow::Result<Option<Cover>> {
	client.get_cover_by_id(id).await
}

#[cached(
	result = true,
	ty = "TimedCache<i64, Option<ExternalGame>>",
	create = "{ TimedCache::with_lifespan(86400) }",
	convert = r#"{ id.clone() }"#
)]
pub async fn get_external_game_by_id_cached(
	client: &IgdbClient,
	id: i64,
) -> anyhow::Result<Option<ExternalGame>> {
	client.get_external_game_by_id(id).await
}

#[cached(
	result = true,
	ty = "TimedCache<i64, Option<Franchise>>",
	create = "{ TimedCache::with_lifespan(86400) }",
	convert = r#"{ id.clone() }"#
)]
pub async fn get_franchise_by_id_cached(
	client: &IgdbClient,
	id: i64,
) -> anyhow::Result<Option<Franchise>> {
	client.get_franchise_by_id(id).await
}

#[cached(
	result = true,
	ty = "TimedCache<i64, Option<Genre>>",
	create = "{ TimedCache::with_lifespan(86400) }",
	convert = r#"{ id.clone() }"#
)]
pub async fn get_genre_by_id_cached(client: &IgdbClient, id: i64) -> anyhow::Result<Option<Genre>> {
	client.get_genre_by_id(id).await
}
