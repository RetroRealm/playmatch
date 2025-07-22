use crate::error::ServiceResult;
use serde::Serialize;
use serde::de::DeserializeOwned;

pub mod identify;
pub mod igdb;

pub const CACHE_PREFIX: &str = "playmatch";

pub trait CacheKey {
	fn get_cache_key(&self, identifier: &str) -> String;
}

fn deserialize_option_redis_value<T: DeserializeOwned>(value: String) -> ServiceResult<Option<T>> {
	let entry: Option<T> = serde_json::from_str(&value)?;
	Ok(entry)
}

fn serialize_option_redis_value<T: Serialize>(value: Option<T>) -> ServiceResult<String> {
	Ok(serde_json::to_string(&value)?)
}
