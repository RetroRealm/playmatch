use crate::metadata::igdb::model::{
	AgeRating, AlternativeName, Artwork, Collection, Cover, ExternalGame, Franchise, Game, Genre,
};
use crate::metadata::igdb::IgdbClient;
use cached::proc_macro::cached;
use cached::TimedSizedCache;

const CACHE_SIZE: usize = 20000;
const CACHE_LIFESPAN: u64 = 86400;
const REFRESH_ON_RETRIEVE: bool = true;

#[cached(
	result = true,
	ty = "TimedSizedCache<i32, Option<Game>>",
	create = "{ TimedSizedCache::with_size_and_lifespan_and_refresh(CACHE_SIZE, CACHE_LIFESPAN, REFRESH_ON_RETRIEVE) }",
	convert = r#"{ id.clone() }"#
)]
pub async fn get_game_by_id_cached(client: &IgdbClient, id: i32) -> anyhow::Result<Option<Game>> {
	client.get_game_by_id(id).await
}

#[cached(
	result = true,
	ty = "TimedSizedCache<String, Vec<Game>>",
	create = "{ TimedSizedCache::with_size_and_lifespan_and_refresh(CACHE_SIZE, CACHE_LIFESPAN, REFRESH_ON_RETRIEVE) }",
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
	ty = "TimedSizedCache<i32, Option<AgeRating>>",
	create = "{ TimedSizedCache::with_size_and_lifespan_and_refresh(CACHE_SIZE, CACHE_LIFESPAN, REFRESH_ON_RETRIEVE) }",
	convert = r#"{ id.clone() }"#
)]
pub async fn get_age_rating_by_id_cached(
	client: &IgdbClient,
	id: i32,
) -> anyhow::Result<Option<AgeRating>> {
	client.get_age_rating_by_id(id).await
}

#[cached(
	result = true,
	ty = "TimedSizedCache<i32, Option<AlternativeName>>",
	create = "{ TimedSizedCache::with_size_and_lifespan_and_refresh(CACHE_SIZE, CACHE_LIFESPAN, REFRESH_ON_RETRIEVE) }",
	convert = r#"{ id.clone() }"#
)]
pub async fn get_alternative_name_by_id_cached(
	client: &IgdbClient,
	id: i32,
) -> anyhow::Result<Option<AlternativeName>> {
	client.get_alternative_name_by_id(id).await
}

#[cached(
	result = true,
	ty = "TimedSizedCache<i32, Option<Artwork>>",
	create = "{ TimedSizedCache::with_size_and_lifespan_and_refresh(CACHE_SIZE, CACHE_LIFESPAN, REFRESH_ON_RETRIEVE) }",
	convert = r#"{ id.clone() }"#
)]
pub async fn get_artwork_by_id_cached(
	client: &IgdbClient,
	id: i32,
) -> anyhow::Result<Option<Artwork>> {
	client.get_artwork_by_id(id).await
}

#[cached(
	result = true,
	ty = "TimedSizedCache<i32, Option<Collection>>",
	create = "{ TimedSizedCache::with_size_and_lifespan_and_refresh(CACHE_SIZE, CACHE_LIFESPAN, REFRESH_ON_RETRIEVE) }",
	convert = r#"{ id.clone() }"#
)]
pub async fn get_collection_by_id_cached(
	client: &IgdbClient,
	id: i32,
) -> anyhow::Result<Option<Collection>> {
	client.get_collection_by_id(id).await
}

#[cached(
	result = true,
	ty = "TimedSizedCache<i32, Option<Cover>>",
	create = "{ TimedSizedCache::with_size_and_lifespan_and_refresh(CACHE_SIZE, CACHE_LIFESPAN, REFRESH_ON_RETRIEVE) }",
	convert = r#"{ id.clone() }"#
)]
pub async fn get_cover_by_id_cached(client: &IgdbClient, id: i32) -> anyhow::Result<Option<Cover>> {
	client.get_cover_by_id(id).await
}

#[cached(
	result = true,
	ty = "TimedSizedCache<i32, Option<ExternalGame>>",
	create = "{ TimedSizedCache::with_size_and_lifespan_and_refresh(CACHE_SIZE, CACHE_LIFESPAN, REFRESH_ON_RETRIEVE) }",
	convert = r#"{ id.clone() }"#
)]
pub async fn get_external_game_by_id_cached(
	client: &IgdbClient,
	id: i32,
) -> anyhow::Result<Option<ExternalGame>> {
	client.get_external_game_by_id(id).await
}

#[cached(
	result = true,
	ty = "TimedSizedCache<i32, Option<Franchise>>",
	create = "{ TimedSizedCache::with_size_and_lifespan_and_refresh(CACHE_SIZE, CACHE_LIFESPAN, REFRESH_ON_RETRIEVE) }",
	convert = r#"{ id.clone() }"#
)]
pub async fn get_franchise_by_id_cached(
	client: &IgdbClient,
	id: i32,
) -> anyhow::Result<Option<Franchise>> {
	client.get_franchise_by_id(id).await
}

#[cached(
	result = true,
	ty = "TimedSizedCache<i32, Option<Genre>>",
	create = "{ TimedSizedCache::with_size_and_lifespan_and_refresh(CACHE_SIZE, CACHE_LIFESPAN, REFRESH_ON_RETRIEVE) }",
	convert = r#"{ id.clone() }"#
)]
pub async fn get_genre_by_id_cached(client: &IgdbClient, id: i32) -> anyhow::Result<Option<Genre>> {
	client.get_genre_by_id(id).await
}
