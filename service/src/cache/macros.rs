//! Provider-agnostic cache wrappers. Each provider defines a thin prefilled
//! wrapper macro in its own `cache.rs` so per-entity invocations stay terse.
//!
//! The macros assume the following are in scope at the invocation site:
//! `redis::{AsyncTypedCommands, Expiry, aio::MultiplexedConnection}` and
//! the `CacheKey` trait from `crate::cache`.

/// L2 (Redis) id-keyed cached lookup. See `providers/igdb/cache.rs` for an
/// IGDB-prefilled wrapper.
#[macro_export]
macro_rules! __cached_lookup_impl {
	(
		$fn_name:ident,
		$client_ty:ty,
		$cache_type:ty,
		$provider_label:literal,
		$lifetime:expr,
		$ty:ty,
		$fetch:ident,
		$variant:ident,
		$label:literal
	) => {
		pub async fn $fn_name(
			client: $client_ty,
			redis_conn: &mut ::redis::aio::MultiplexedConnection,
			id: i32,
		) -> ::anyhow::Result<Option<$ty>> {
			let cache_key = <$cache_type>::$variant.get_cache_key(&id.to_string());

			if let Ok(Some(cached_val)) = redis_conn
				.get_ex(&cache_key, ::redis::Expiry::EX($lifetime))
				.await
			{
				::log::debug!(
					"{} cache hit for {} with id: {}",
					$provider_label,
					$label,
					id
				);
				$crate::metrics::record_cache_hit($provider_label, $label);
				let deserialized = $crate::cache::deserialize_option_redis_value(cached_val)?;
				return Ok(deserialized);
			}
			::log::debug!(
				"{} cache miss for {} with id: {}",
				$provider_label,
				$label,
				id
			);
			$crate::metrics::record_cache_miss($provider_label, $label);

			let value = client.$fetch(id).await?;

			let payload = $crate::cache::serialize_option_redis_value(value.clone())?;
			$crate::cache::spawn_cache_write(redis_conn.clone(), cache_key, payload, $lifetime);

			Ok(value)
		}
	};
}

/// L1 (in-process moka) + L2 (Redis) id-keyed cached lookup. Reserved for small,
/// rarely-changing reference entities where a per-worker clone is negligible.
#[macro_export]
macro_rules! __cached_reference_lookup_impl {
	(
		$fn_name:ident,
		$client_ty:ty,
		$cache_type:ty,
		$provider_label:literal,
		$l1_label:literal,
		$l1_capacity:expr,
		$l1_time_to_idle:expr,
		$lifetime:expr,
		$ty:ty,
		$fetch:ident,
		$variant:ident,
		$label:literal
	) => {
		pub async fn $fn_name(
			client: $client_ty,
			redis_conn: &mut ::redis::aio::MultiplexedConnection,
			id: i32,
		) -> ::anyhow::Result<Option<$ty>> {
			static L1: ::std::sync::OnceLock<::moka::future::Cache<i32, Option<$ty>>> =
				::std::sync::OnceLock::new();
			let l1 = L1.get_or_init(|| {
				::moka::future::Cache::builder()
					.max_capacity($l1_capacity)
					.time_to_idle($l1_time_to_idle)
					.build()
			});

			if let Some(hit) = l1.get(&id).await {
				::log::debug!("{} L1 hit for {} with id: {}", $provider_label, $label, id);
				$crate::metrics::record_cache_hit($l1_label, $label);
				return Ok(hit);
			}
			::log::debug!("{} L1 miss for {} with id: {}", $provider_label, $label, id);
			$crate::metrics::record_cache_miss($l1_label, $label);

			let cache_key = <$cache_type>::$variant.get_cache_key(&id.to_string());

			if let Ok(Some(cached_val)) = redis_conn
				.get_ex(&cache_key, ::redis::Expiry::EX($lifetime))
				.await
			{
				::log::debug!("{} L2 hit for {} with id: {}", $provider_label, $label, id);
				$crate::metrics::record_cache_hit($provider_label, $label);
				let deserialized: Option<$ty> =
					$crate::cache::deserialize_option_redis_value(cached_val)?;
				l1.insert(id, deserialized.clone()).await;
				$crate::metrics::set_cache_l1_entries($label, l1.entry_count());
				return Ok(deserialized);
			}
			::log::debug!(
				"{} cache miss for {} with id: {}",
				$provider_label,
				$label,
				id
			);
			$crate::metrics::record_cache_miss($provider_label, $label);

			let value = client.$fetch(id).await?;

			l1.insert(id, value.clone()).await;
			$crate::metrics::set_cache_l1_entries($label, l1.entry_count());
			let payload = $crate::cache::serialize_option_redis_value(value.clone())?;
			$crate::cache::spawn_cache_write(redis_conn.clone(), cache_key, payload, $lifetime);

			Ok(value)
		}
	};
}

/// L2 (Redis) slug-keyed cached lookup.
#[macro_export]
macro_rules! __cached_lookup_by_slug_impl {
	(
		$fn_name:ident,
		$client_ty:ty,
		$cache_type:ty,
		$provider_label:literal,
		$lifetime:expr,
		$ty:ty,
		$fetch:ident,
		$variant:ident,
		$label:literal
	) => {
		pub async fn $fn_name(
			client: $client_ty,
			redis_conn: &mut ::redis::aio::MultiplexedConnection,
			slug: String,
		) -> ::anyhow::Result<Option<$ty>> {
			let cache_key =
				<$cache_type>::$variant.get_cache_key(&$crate::cache::normalised_key_hash(&slug));

			if let Ok(Some(cached_val)) = redis_conn
				.get_ex(&cache_key, ::redis::Expiry::EX($lifetime))
				.await
			{
				::log::debug!(
					"{} cache hit for {} with slug: {}",
					$provider_label,
					$label,
					slug
				);
				$crate::metrics::record_cache_hit($provider_label, $label);
				let deserialized = $crate::cache::deserialize_option_redis_value(cached_val)?;
				return Ok(deserialized);
			}
			::log::debug!(
				"{} cache miss for {} with slug: {}",
				$provider_label,
				$label,
				slug
			);
			$crate::metrics::record_cache_miss($provider_label, $label);

			let value = client.$fetch(&slug).await?;

			let payload = $crate::cache::serialize_option_redis_value(value.clone())?;
			$crate::cache::spawn_cache_write(redis_conn.clone(), cache_key, payload, $lifetime);

			Ok(value)
		}
	};
}

/// L2 (Redis) free-text search cached lookup. Returns `Vec<$ty>` rather than
/// `Option<$ty>`.
#[macro_export]
macro_rules! __cached_search_impl {
	(
		$fn_name:ident,
		$client_ty:ty,
		$cache_type:ty,
		$provider_label:literal,
		$lifetime:expr,
		$ty:ty,
		$fetch:ident,
		$variant:ident,
		$label:literal
	) => {
		pub async fn $fn_name(
			client: $client_ty,
			redis_conn: &mut ::redis::aio::MultiplexedConnection,
			query: String,
		) -> ::anyhow::Result<Vec<$ty>> {
			let cache_key =
				<$cache_type>::$variant.get_cache_key(&$crate::cache::normalised_key_hash(&query));

			if let Ok(Some(cached_val)) = redis_conn
				.get_ex(&cache_key, ::redis::Expiry::EX($lifetime))
				.await
			{
				::log::debug!(
					"{} cache hit for {} query: {}",
					$provider_label,
					$label,
					query
				);
				$crate::metrics::record_cache_hit($provider_label, $label);
				let deserialized: Vec<$ty> = ::serde_json::from_str(&cached_val)?;
				return Ok(deserialized);
			}
			::log::debug!(
				"{} cache miss for {} query: {}",
				$provider_label,
				$label,
				query
			);
			$crate::metrics::record_cache_miss($provider_label, $label);

			let values = client.$fetch(&query).await?;

			let payload = ::serde_json::to_string(&values)?;
			$crate::cache::spawn_cache_write(redis_conn.clone(), cache_key, payload, $lifetime);

			Ok(values)
		}
	};
}
