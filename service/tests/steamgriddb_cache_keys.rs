//! Pins the exact Redis key layout the SteamGridDB cache wrappers emit.
//! If any of these assertions break you have invalidated every existing
//! cached entry; bump `CACHE_KEY_VERSION` in `service/src/cache/mod.rs`
//! deliberately rather than changing the key shape.

use service::cache::provider_cache_key;
use service::providers::steamgriddb::model::AssetFilters;

#[test]
fn sgdb_game_by_id_layout_is_stable() {
	assert_eq!(
		provider_cache_key("steamgriddb", "game", "12345"),
		"playmatch:cache:v1:steamgriddb:game:12345",
	);
}

#[test]
fn sgdb_game_by_platform_layout_is_stable() {
	assert_eq!(
		provider_cache_key("steamgriddb", "game:platform", "steam:730"),
		"playmatch:cache:v1:steamgriddb:game:platform:steam:730",
	);
}

#[test]
fn sgdb_game_search_layout_is_stable() {
	// The trailing component is a sha256 of the normalised query; assert the
	// surrounding shape only so we can change the term without churning here.
	assert!(
		provider_cache_key("steamgriddb", "game:search", "deadbeef")
			.starts_with("playmatch:cache:v1:steamgriddb:game:search:")
	);
}

#[test]
fn sgdb_asset_segments_are_stable() {
	for segment in [
		"grid:game",
		"grid:platform",
		"hero:game",
		"hero:platform",
		"logo:game",
		"logo:platform",
		"icon:game",
		"icon:platform",
	] {
		assert!(
			provider_cache_key("steamgriddb", segment, "x")
				.starts_with(&format!("playmatch:cache:v1:steamgriddb:{segment}:")),
			"unexpected layout for segment {segment}"
		);
	}
}

#[test]
fn empty_filters_canonicalise_to_empty_string() {
	let filters = AssetFilters::default();
	assert_eq!(filters.canonical_string(), "");
}

#[test]
fn filter_canonical_string_is_sorted_and_lowercased() {
	let filters = AssetFilters {
		styles: vec!["Alternate".into(), "blurred".into()],
		limit: Some(10),
		page: Some(2),
		..Default::default()
	};
	assert_eq!(
		filters.canonical_string(),
		"limit=10&page=2&styles=alternate,blurred",
	);
}

#[test]
fn filter_canonical_string_is_deterministic() {
	let a = AssetFilters {
		mimes: vec!["image/png".into(), "image/webp".into()],
		dimensions: vec!["460x215".into()],
		..Default::default()
	};
	let b = AssetFilters {
		dimensions: vec!["460x215".into()],
		mimes: vec!["image/png".into(), "image/webp".into()],
		..Default::default()
	};
	assert_eq!(a.canonical_string(), b.canonical_string());
}
