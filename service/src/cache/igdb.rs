use crate::cache::{
	CACHE_PREFIX, CacheKey, deserialize_option_redis_value, serialize_option_redis_value,
};
use crate::metadata::igdb::IgdbClient;
use crate::metadata::igdb::model::{
	AgeRating, AlternativeName, Artwork, Collection, Cover, ExternalGame, Franchise, Game, Genre,
};
use log::debug;
use redis::AsyncTypedCommands;
use redis::aio::MultiplexedConnection;
use std::time::Duration;

const IGDB_CACHE_LIFETIME: u64 = Duration::from_secs(60 * 60 * 24).as_secs(); // 1 day

pub async fn get_game_by_id_cached(
	igdb_client: &IgdbClient,
	redis_conn: &mut MultiplexedConnection,
	id: i32,
) -> anyhow::Result<Option<Game>> {
	let cache_key = IgdbCacheType::GetIgdbGameById.get_cache_key(&id.to_string());

	if let Ok(Some(cached_val)) = redis_conn.get(&cache_key).await {
		debug!("igdb Cache hit for Game with id: {id}");
		redis_conn
			.expire(&cache_key, IGDB_CACHE_LIFETIME as i64)
			.await?;
		let deserialized = deserialize_option_redis_value(cached_val)?;
		return Ok(deserialized);
	}
	debug!("igdb Cache miss for Game with id: {id}");

	let game = igdb_client.get_game_by_id(id).await?;

	redis_conn
		.set_ex(
			&cache_key,
			serialize_option_redis_value(game.clone())?,
			IGDB_CACHE_LIFETIME,
		)
		.await?;

	Ok(game)
}

pub async fn get_game_by_slug_cached(
	igdb_client: &IgdbClient,
	redis_conn: &mut MultiplexedConnection,
	slug: String,
) -> anyhow::Result<Option<Game>> {
	let cache_key = IgdbCacheType::GetIgdbGameBySlug.get_cache_key(&slug);

	if let Ok(Some(cached_val)) = redis_conn.get(&cache_key).await {
		debug!("igdb Cache hit for Game with slug: {slug}");
		redis_conn
			.expire(&cache_key, IGDB_CACHE_LIFETIME as i64)
			.await?;
		let deserialized = deserialize_option_redis_value(cached_val)?;
		return Ok(deserialized);
	}
	debug!("igdb Cache miss for Game with slug: {slug}");

	let game = igdb_client.get_game_by_slug(&slug).await?;

	redis_conn
		.set_ex(
			&cache_key,
			serialize_option_redis_value(game.clone())?,
			IGDB_CACHE_LIFETIME,
		)
		.await?;

	Ok(game)
}

pub async fn search_game_by_name_cached(
	igdb_client: &IgdbClient,
	redis_conn: &mut MultiplexedConnection,
	query: String,
) -> anyhow::Result<Vec<Game>> {
	let cache_key = IgdbCacheType::SearchIgdbGameByName.get_cache_key(&query);

	if let Ok(Some(cached_val)) = redis_conn.get(&cache_key).await {
		debug!("igdb Cache hit for Search Game by Name: {query}");
		redis_conn
			.expire(&cache_key, IGDB_CACHE_LIFETIME as i64)
			.await?;
		let deserialized: Vec<Game> = serde_json::from_str(&cached_val)?;
		return Ok(deserialized);
	}
	debug!("igdb Cache miss for Search Game by Name: {query}");

	let games = igdb_client.search_game_by_name(&query).await?;

	redis_conn
		.set_ex(
			&cache_key,
			serde_json::to_string(&games)?,
			IGDB_CACHE_LIFETIME,
		)
		.await?;

	Ok(games)
}

pub async fn get_age_rating_by_id_cached(
	igdb_client: &IgdbClient,
	redis_conn: &mut MultiplexedConnection,
	id: i32,
) -> anyhow::Result<Option<AgeRating>> {
	let cache_key = IgdbCacheType::GetAgeRatingById.get_cache_key(&id.to_string());

	if let Ok(Some(cached_val)) = redis_conn.get(&cache_key).await {
		debug!("igdb Cache hit for Age Rating with id: {id}");
		redis_conn
			.expire(&cache_key, IGDB_CACHE_LIFETIME as i64)
			.await?;
		let deserialized = deserialize_option_redis_value(cached_val)?;
		return Ok(deserialized);
	}
	debug!("igdb Cache miss for Age Rating with id: {id}");

	let age_rating = igdb_client.get_age_rating_by_id(id).await?;

	redis_conn
		.set_ex(
			&cache_key,
			serialize_option_redis_value(age_rating.clone())?,
			IGDB_CACHE_LIFETIME,
		)
		.await?;

	Ok(age_rating)
}

pub async fn get_alternative_name_by_id_cached(
	igdb_client: &IgdbClient,
	redis_conn: &mut MultiplexedConnection,
	id: i32,
) -> anyhow::Result<Option<AlternativeName>> {
	let cache_key = IgdbCacheType::GetAlternativeNameById.get_cache_key(&id.to_string());

	if let Ok(Some(cached_val)) = redis_conn.get(&cache_key).await {
		debug!("igdb Cache hit for Alternative Name with id: {id}");
		redis_conn
			.expire(&cache_key, IGDB_CACHE_LIFETIME as i64)
			.await?;
		let deserialized = deserialize_option_redis_value(cached_val)?;
		return Ok(deserialized);
	}
	debug!("igdb Cache miss for Alternative Name with id: {id}");

	let alternative_name = igdb_client.get_alternative_name_by_id(id).await?;

	redis_conn
		.set_ex(
			&cache_key,
			serialize_option_redis_value(alternative_name.clone())?,
			IGDB_CACHE_LIFETIME,
		)
		.await?;

	Ok(alternative_name)
}

pub async fn get_artwork_by_id_cached(
	igdb_client: &IgdbClient,
	redis_conn: &mut MultiplexedConnection,
	id: i32,
) -> anyhow::Result<Option<Artwork>> {
	let cache_key = IgdbCacheType::GetArtworkById.get_cache_key(&id.to_string());

	if let Ok(Some(cached_val)) = redis_conn.get(&cache_key).await {
		debug!("igdb Cache hit for Artwork with id: {id}");
		redis_conn
			.expire(&cache_key, IGDB_CACHE_LIFETIME as i64)
			.await?;
		let deserialized = deserialize_option_redis_value(cached_val)?;
		return Ok(deserialized);
	}
	debug!("igdb Cache miss for Artwork with id: {id}");

	let artwork = igdb_client.get_artwork_by_id(id).await?;

	redis_conn
		.set_ex(
			&cache_key,
			serialize_option_redis_value(artwork.clone())?,
			IGDB_CACHE_LIFETIME,
		)
		.await?;

	Ok(artwork)
}

pub async fn get_collection_by_id_cached(
	igdb_client: &IgdbClient,
	redis_conn: &mut MultiplexedConnection,
	id: i32,
) -> anyhow::Result<Option<Collection>> {
	let cache_key = IgdbCacheType::GetCollectionById.get_cache_key(&id.to_string());

	if let Ok(Some(cached_val)) = redis_conn.get(&cache_key).await {
		debug!("igdb Cache hit for Collection with id: {id}");
		redis_conn
			.expire(&cache_key, IGDB_CACHE_LIFETIME as i64)
			.await?;
		let deserialized = deserialize_option_redis_value(cached_val)?;
		return Ok(deserialized);
	}
	debug!("igdb Cache miss for Collection with id: {id}");

	let collection = igdb_client.get_collection_by_id(id).await?;

	redis_conn
		.set_ex(
			&cache_key,
			serialize_option_redis_value(collection.clone())?,
			IGDB_CACHE_LIFETIME,
		)
		.await?;

	Ok(collection)
}

pub async fn get_cover_by_id_cached(
	igdb_client: &IgdbClient,
	redis_conn: &mut MultiplexedConnection,
	id: i32,
) -> anyhow::Result<Option<Cover>> {
	let cache_key = IgdbCacheType::GetCoverById.get_cache_key(&id.to_string());

	if let Ok(Some(cached_val)) = redis_conn.get(&cache_key).await {
		debug!("igdb Cache hit for Cover with id: {id}");
		redis_conn
			.expire(&cache_key, IGDB_CACHE_LIFETIME as i64)
			.await?;
		let deserialized = deserialize_option_redis_value(cached_val)?;
		return Ok(deserialized);
	}
	debug!("igdb Cache miss for Cover with id: {id}");

	let cover = igdb_client.get_cover_by_id(id).await?;

	redis_conn
		.set_ex(
			&cache_key,
			serialize_option_redis_value(cover.clone())?,
			IGDB_CACHE_LIFETIME,
		)
		.await?;

	Ok(cover)
}

pub async fn get_external_game_by_id_cached(
	igdb_client: &IgdbClient,
	redis_conn: &mut MultiplexedConnection,
	id: i32,
) -> anyhow::Result<Option<ExternalGame>> {
	let cache_key = IgdbCacheType::GetExternalGameById.get_cache_key(&id.to_string());

	if let Ok(Some(cached_val)) = redis_conn.get(&cache_key).await {
		debug!("igdb Cache hit for External Game with id: {id}");
		redis_conn
			.expire(&cache_key, IGDB_CACHE_LIFETIME as i64)
			.await?;
		let deserialized = deserialize_option_redis_value(cached_val)?;
		return Ok(deserialized);
	}
	debug!("igdb Cache miss for External Game with id: {id}");

	let external_game = igdb_client.get_external_game_by_id(id).await?;

	redis_conn
		.set_ex(
			&cache_key,
			serialize_option_redis_value(external_game.clone())?,
			IGDB_CACHE_LIFETIME,
		)
		.await?;

	Ok(external_game)
}

pub async fn get_franchise_by_id_cached(
	igdb_client: &IgdbClient,
	redis_conn: &mut MultiplexedConnection,
	id: i32,
) -> anyhow::Result<Option<Franchise>> {
	let cache_key = IgdbCacheType::GetFranchiseById.get_cache_key(&id.to_string());

	if let Ok(Some(cached_val)) = redis_conn.get(&cache_key).await {
		debug!("igdb Cache hit for Franchise with id: {id}");
		redis_conn
			.expire(&cache_key, IGDB_CACHE_LIFETIME as i64)
			.await?;
		let deserialized = deserialize_option_redis_value(cached_val)?;
		return Ok(deserialized);
	}
	debug!("igdb Cache miss for Franchise with id: {id}");

	let franchise = igdb_client.get_franchise_by_id(id).await?;

	redis_conn
		.set_ex(
			&cache_key,
			serialize_option_redis_value(franchise.clone())?,
			IGDB_CACHE_LIFETIME,
		)
		.await?;

	Ok(franchise)
}

pub async fn get_genre_by_id_cached(
	igdb_client: &IgdbClient,
	redis_conn: &mut MultiplexedConnection,
	id: i32,
) -> anyhow::Result<Option<Genre>> {
	let cache_key = IgdbCacheType::GetGenreById.get_cache_key(&id.to_string());

	if let Ok(Some(cached_val)) = redis_conn.get(&cache_key).await {
		debug!("igdb Cache hit for Genre with id: {id}");
		redis_conn
			.expire(&cache_key, IGDB_CACHE_LIFETIME as i64)
			.await?;
		let deserialized = deserialize_option_redis_value(cached_val)?;
		return Ok(deserialized);
	}
	debug!("igdb Cache miss for Genre with id: {id}");

	let genre = igdb_client.get_genre_by_id(id).await?;

	redis_conn
		.set_ex(
			&cache_key,
			serialize_option_redis_value(genre.clone())?,
			IGDB_CACHE_LIFETIME,
		)
		.await?;

	Ok(genre)
}

#[derive(Debug, Clone, Copy)]
enum IgdbCacheType {
	GetIgdbGameById,
	GetIgdbGameBySlug,
	SearchIgdbGameByName,
	GetAgeRatingById,
	GetAlternativeNameById,
	GetArtworkById,
	GetCollectionById,
	GetCoverById,
	GetExternalGameById,
	GetFranchiseById,
	GetGenreById,
}

impl CacheKey for IgdbCacheType {
	fn get_cache_key(&self, identifier: &str) -> String {
		match self {
			IgdbCacheType::GetIgdbGameById => {
				format!("{CACHE_PREFIX}:cache:igdb:game:{identifier}")
			}
			IgdbCacheType::GetIgdbGameBySlug => {
				format!("{CACHE_PREFIX}:cache:igdb:game:slug:{identifier}")
			}
			IgdbCacheType::SearchIgdbGameByName => {
				format!("{CACHE_PREFIX}:cache:igdb:game:search:{identifier}")
			}
			IgdbCacheType::GetAgeRatingById => {
				format!("{CACHE_PREFIX}:cache:igdb:age_rating:{identifier}")
			}
			IgdbCacheType::GetAlternativeNameById => {
				format!("{CACHE_PREFIX}:cache:igdb:alternative_name:{identifier}")
			}
			IgdbCacheType::GetArtworkById => {
				format!("{CACHE_PREFIX}:cache:igdb:artwork:{identifier}")
			}
			IgdbCacheType::GetCollectionById => {
				format!("{CACHE_PREFIX}:cache:igdb:collection:{identifier}")
			}
			IgdbCacheType::GetCoverById => {
				format!("{CACHE_PREFIX}:cache:igdb:cover:{identifier}")
			}
			IgdbCacheType::GetExternalGameById => {
				format!("{CACHE_PREFIX}:cache:igdb:external_game:{identifier}")
			}
			IgdbCacheType::GetFranchiseById => {
				format!("{CACHE_PREFIX}:cache:igdb:franchise:{identifier}")
			}
			IgdbCacheType::GetGenreById => {
				format!("{CACHE_PREFIX}:cache:igdb:genre:{identifier}")
			}
		}
	}
}
