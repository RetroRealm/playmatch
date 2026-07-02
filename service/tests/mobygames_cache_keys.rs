//! Pins the exact Redis key layout the MobyGames cache wrappers emit.
//! If any of these assertions break you have invalidated every existing
//! cached entry; bump `CACHE_KEY_VERSION` in `service/src/cache/mod.rs`
//! deliberately rather than changing the key shape.

use service::cache::provider_cache_key;

#[test]
fn mg_platforms_layout_is_stable() {
	assert_eq!(
		provider_cache_key("mobygames", "platforms", "all"),
		"playmatch:cache:v2:mobygames:platforms:all",
	);
}

#[test]
fn mg_genres_layout_is_stable() {
	assert_eq!(
		provider_cache_key("mobygames", "genres", "all"),
		"playmatch:cache:v2:mobygames:genres:all",
	);
}

#[test]
fn mg_game_by_id_layout_is_stable() {
	assert_eq!(
		provider_cache_key("mobygames", "game", "12345"),
		"playmatch:cache:v2:mobygames:game:12345",
	);
}

#[test]
fn mg_game_search_layout_is_stable() {
	// The trailing component is a sha256 of the normalized query; assert the
	// surrounding shape only.
	assert!(
		provider_cache_key("mobygames", "game:search", "deadbeef")
			.starts_with("playmatch:cache:v2:mobygames:game:search:")
	);
}

#[test]
fn mg_game_search_with_platform_layout_is_stable() {
	assert!(
		provider_cache_key("mobygames", "game:search", "platform:5:deadbeef")
			.starts_with("playmatch:cache:v2:mobygames:game:search:platform:")
	);
}

#[test]
fn mg_game_covers_layout_is_stable() {
	assert_eq!(
		provider_cache_key("mobygames", "game:covers", "12345:5"),
		"playmatch:cache:v2:mobygames:game:covers:12345:5",
	);
}

#[test]
fn mg_game_screenshots_layout_is_stable() {
	assert_eq!(
		provider_cache_key("mobygames", "game:screenshots", "12345:5"),
		"playmatch:cache:v2:mobygames:game:screenshots:12345:5",
	);
}
