//! Pins the exact Redis key layout the LaunchBox cache wrappers emit.
//! If any of these assertions break you have invalidated every existing
//! cached entry; bump `CACHE_KEY_VERSION` in `service/src/cache/mod.rs`
//! deliberately rather than changing the key shape.

use service::cache::provider_cache_key;

#[test]
fn lb_platforms_layout_is_stable() {
	assert_eq!(
		provider_cache_key("launchbox", "platforms", "all"),
		"playmatch:cache:v2:launchbox:platforms:all",
	);
}

#[test]
fn lb_game_by_id_layout_is_stable() {
	assert_eq!(
		provider_cache_key("launchbox", "game", "12345"),
		"playmatch:cache:v2:launchbox:game:12345",
	);
}

#[test]
fn lb_game_search_layout_is_stable() {
	assert!(
		provider_cache_key("launchbox", "game:search", "deadbeef")
			.starts_with("playmatch:cache:v2:launchbox:game:search:")
	);
}

#[test]
fn lb_game_search_with_platform_layout_is_stable() {
	assert!(
		provider_cache_key("launchbox", "game:search", "platform:nes:deadbeef")
			.starts_with("playmatch:cache:v2:launchbox:game:search:platform:")
	);
}

#[test]
fn lb_game_alternate_names_layout_is_stable() {
	assert_eq!(
		provider_cache_key("launchbox", "game:alternate-names", "12345"),
		"playmatch:cache:v2:launchbox:game:alternate-names:12345",
	);
}

#[test]
fn lb_game_images_layout_is_stable() {
	assert_eq!(
		provider_cache_key("launchbox", "game:images", "12345"),
		"playmatch:cache:v2:launchbox:game:images:12345",
	);
}
