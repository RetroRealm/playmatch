use crate::error::ServiceResult;
use serde::Serialize;
use serde::de::DeserializeOwned;

pub const CACHE_PREFIX: &str = "playmatch";

#[derive(Debug, Clone)]
pub enum CacheStatus<T> {
	Cached(T),
	NonCached(T),
}

pub trait CacheKey {
	fn get_cache_key(&self, identifier: &str) -> String;
}

pub(crate) fn deserialize_option_redis_value<T: DeserializeOwned>(
	value: String,
) -> ServiceResult<Option<T>> {
	let entry: Option<T> = serde_json::from_str(&value)?;
	Ok(entry)
}

pub(crate) fn serialize_option_redis_value<T: Serialize>(
	value: Option<T>,
) -> ServiceResult<String> {
	Ok(serde_json::to_string(&value)?)
}
