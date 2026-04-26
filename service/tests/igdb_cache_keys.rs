//! Pins the exact Redis key layout the IGDB cache macros emit. If any of these
//! assertions break you have invalidated every existing cached entry; bump
//! `CACHE_KEY_VERSION` in `service/src/cache/mod.rs` deliberately rather than
//! changing the key shape.

use service::cache::provider_cache_key;

#[test]
fn igdb_id_keyed_layout_is_stable() {
	assert_eq!(
		provider_cache_key("igdb", "age_rating", "42"),
		"playmatch:cache:v1:igdb:age_rating:42",
	);
	assert_eq!(
		provider_cache_key("igdb", "company", "1"),
		"playmatch:cache:v1:igdb:company:1",
	);
	assert_eq!(
		provider_cache_key("igdb", "platform_version_release_date", "9999"),
		"playmatch:cache:v1:igdb:platform_version_release_date:9999",
	);
}

#[test]
fn igdb_compound_segments_preserve_embedded_colons() {
	// `game:slug` and `game:search` are the only segments with an embedded
	// colon. Their layout must remain bit-identical across refactors.
	assert_eq!(
		provider_cache_key("igdb", "game:slug", "abcdef0123"),
		"playmatch:cache:v1:igdb:game:slug:abcdef0123",
	);
	assert_eq!(
		provider_cache_key("igdb", "game:search", "deadbeef"),
		"playmatch:cache:v1:igdb:game:search:deadbeef",
	);
}

#[test]
fn provider_segment_appears_after_version() {
	assert!(provider_cache_key("foo", "game", "1").starts_with("playmatch:cache:v1:foo:game:"));
}
