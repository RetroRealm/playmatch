use crate::db::game::{
	find_game_and_id_mapping_by_md5, find_game_and_id_mapping_by_sha1,
	find_game_and_id_mapping_by_sha256,
};
use cached::TimedSizedCache;
use cached::proc_macro::cached;
use entity::{game, signature_metadata_mapping};
use sea_orm::{DbConn, DbErr};

const CACHE_SIZE: usize = 1_000_000; // 1 million entries
const CACHE_LIFESPAN: u64 = 86400; // 24 hours in seconds
const REFRESH_ON_RETRIEVE: bool = true;

#[cached(
	result = true,
	ty = "TimedSizedCache<String, Option<(game::Model, Vec<signature_metadata_mapping::Model>)>>",
	create = "{ TimedSizedCache::with_size_and_lifespan_and_refresh(CACHE_SIZE, CACHE_LIFESPAN, REFRESH_ON_RETRIEVE) }",
	convert = r#"{ sha256.to_string() }"#
)]
pub async fn find_game_and_id_mapping_by_sha256_cached(
	sha256: &str,
	conn: &DbConn,
) -> Result<Option<(game::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	find_game_and_id_mapping_by_sha256(sha256, conn).await
}

#[cached(
	result = true,
	ty = "TimedSizedCache<String, Option<(game::Model, Vec<signature_metadata_mapping::Model>)>>",
	create = "{ TimedSizedCache::with_size_and_lifespan_and_refresh(CACHE_SIZE, CACHE_LIFESPAN, REFRESH_ON_RETRIEVE) }",
	convert = r#"{ sha1.to_string() }"#
)]
pub async fn find_game_and_id_mapping_by_sha1_cached(
	sha1: &str,
	conn: &DbConn,
) -> Result<Option<(game::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	find_game_and_id_mapping_by_sha1(sha1, conn).await
}

#[cached(
	result = true,
	ty = "TimedSizedCache<String, Option<(game::Model, Vec<signature_metadata_mapping::Model>)>>",
	create = "{ TimedSizedCache::with_size_and_lifespan_and_refresh(CACHE_SIZE, CACHE_LIFESPAN, REFRESH_ON_RETRIEVE) }",
	convert = r#"{ md5.to_string() }"#
)]
pub async fn find_game_and_id_mapping_by_md5_cached(
	md5: &str,
	conn: &DbConn,
) -> Result<Option<(game::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	find_game_and_id_mapping_by_md5(md5, conn).await
}
