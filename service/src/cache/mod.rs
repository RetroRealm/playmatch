mod macros;

use crate::error::ServiceResult;
use log::warn;
use redis::AsyncTypedCommands;
use redis::aio::MultiplexedConnection;
use serde::Serialize;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};

pub const CACHE_PREFIX: &str = "playmatch";

/// Bump on any backwards-incompatible change to cached payload shapes. All
/// cache keys carry this segment so a version change naturally invalidates
/// legacy entries as they age out via TTL; the new code reads and writes the
/// bumped namespace immediately.
pub const CACHE_KEY_VERSION: &str = "v1";

#[derive(Debug, Clone)]
pub enum CacheStatus<T> {
	Cached(T),
	NonCached(T),
}

pub trait CacheKey {
	fn get_cache_key(&self, identifier: &str) -> String;
}

/// Format the canonical provider cache key. Layout:
/// `playmatch:cache:v1:<provider>:<segment>:<identifier>`.
pub fn provider_cache_key(provider: &str, segment: &str, identifier: &str) -> String {
	format!("{CACHE_PREFIX}:cache:{CACHE_KEY_VERSION}:{provider}:{segment}:{identifier}")
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

/// Fire-and-forget a SET_EX on a detached task so the request path is not
/// blocked by the cache write. Errors are logged at warn level; the caller
/// already has the value.
pub(crate) fn spawn_cache_write(
	mut redis_conn: MultiplexedConnection,
	cache_key: String,
	payload: String,
	ttl_secs: u64,
) {
	tokio::spawn(async move {
		if let Err(e) = redis_conn.set_ex(&cache_key, payload, ttl_secs).await {
			warn!("cache write failed for {cache_key}: {e}");
		}
	});
}

/// Normalise free-text input before using it as a cache key suffix. Lowercase,
/// trim, and collapse internal whitespace so `"Pokemon"`, `" pokemon "`, and
/// `"POKEMON"` all hit the same entry. The return value is a sha256 hex digest
/// of the normalised form, keeping the key short and free of `:` separators
/// that could collide with the namespace layout.
pub fn normalised_key_hash(input: &str) -> String {
	let mut normalised = String::with_capacity(input.len());
	let mut previous_was_space = true;
	for ch in input.trim().chars().flat_map(char::to_lowercase) {
		if ch.is_whitespace() {
			if !previous_was_space {
				normalised.push(' ');
				previous_was_space = true;
			}
		} else {
			normalised.push(ch);
			previous_was_space = false;
		}
	}
	if normalised.ends_with(' ') {
		normalised.pop();
	}
	hex::encode(Sha256::digest(normalised.as_bytes()))
}

#[cfg(test)]
mod tests {
	use super::normalised_key_hash;

	#[test]
	fn normalisation_collapses_case_and_whitespace() {
		let a = normalised_key_hash("Pokemon");
		let b = normalised_key_hash(" pokemon ");
		let c = normalised_key_hash("POKEMON");
		let d = normalised_key_hash("pok emon");
		assert_eq!(a, b);
		assert_eq!(a, c);
		assert_ne!(a, d);
	}

	#[test]
	fn different_inputs_diverge() {
		assert_ne!(normalised_key_hash("mario"), normalised_key_hash("luigi"));
	}
}
