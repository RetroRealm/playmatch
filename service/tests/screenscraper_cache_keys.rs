//! Pins the exact Redis key layout the ScreenScraper cache wrappers emit.
//! If any of these assertions break you have invalidated every existing
//! cached entry; bump `CACHE_KEY_VERSION` in `service/src/cache/mod.rs`
//! deliberately rather than changing the key shape.

use service::cache::provider_cache_key;

#[test]
fn ss_systems_layout_is_stable() {
	assert_eq!(
		provider_cache_key("screenscraper", "systems", "all"),
		"playmatch:cache:v1:screenscraper:systems:all",
	);
}

#[test]
fn ss_game_by_id_layout_is_stable() {
	assert_eq!(
		provider_cache_key("screenscraper", "game", "12345"),
		"playmatch:cache:v1:screenscraper:game:12345",
	);
}

#[test]
fn ss_game_by_rom_layout_is_stable() {
	// The trailing component is a sha256 of the normalised rom name; assert
	// the surrounding shape only.
	assert!(
		provider_cache_key("screenscraper", "game:rom", "1:deadbeef")
			.starts_with("playmatch:cache:v1:screenscraper:game:rom:")
	);
}

#[test]
fn ss_game_search_layout_is_stable() {
	assert!(
		provider_cache_key("screenscraper", "game:search", "1:deadbeef")
			.starts_with("playmatch:cache:v1:screenscraper:game:search:")
	);
}

#[test]
fn ss_hash_segments_are_stable() {
	for segment in ["game:md5", "game:sha1", "game:crc"] {
		assert!(
			provider_cache_key("screenscraper", segment, "1:abc")
				.starts_with(&format!("playmatch:cache:v1:screenscraper:{segment}:")),
			"unexpected layout for segment {segment}"
		);
	}
}
