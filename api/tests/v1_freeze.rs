//! v1 freeze guards. These pin the externally visible v1 surface so an
//! accidental change to a shared DTO or route fails loudly instead of silently
//! shipping. To intentionally re-baseline after a reviewed change, run:
//!   UPDATE_GOLDEN=1 cargo test -p api --test v1_freeze

use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;

fn golden_path(name: &str) -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("tests")
		.join("golden")
		.join(name)
}

fn assert_or_update_golden(name: &str, actual: &str) {
	let path = golden_path(name);
	if std::env::var("UPDATE_GOLDEN").is_ok() {
		fs::create_dir_all(path.parent().unwrap()).unwrap();
		fs::write(&path, actual).unwrap();
		return;
	}
	let expected = fs::read_to_string(&path).unwrap_or_else(|_| {
		panic!(
			"missing golden {name}; regenerate with UPDATE_GOLDEN=1 cargo test -p api --test v1_freeze"
		)
	});
	assert_eq!(
		expected.trim_end(),
		actual.trim_end(),
		"v1 surface drifted from the frozen golden {name}. If this change is intentional and \
		 reviewed, re-baseline with UPDATE_GOLDEN=1 cargo test -p api --test v1_freeze"
	);
}

fn canonical(value: &Value) -> String {
	// serde_json::Value backed by a BTreeMap serializes object keys sorted, so
	// schema/path ordering is stable regardless of registration order.
	serde_json::to_string_pretty(value).unwrap()
}

/// Structural guard: freezes the v1 component schemas and path set. An additive
/// field on any shared DTO changes a schema object here and fails the diff.
#[test]
fn v1_openapi_structure_is_frozen() {
	let openapi = api::create_openapi();
	let doc: Value = serde_json::to_value(&openapi).unwrap();

	let schemas = doc
		.get("components")
		.and_then(|c| c.get("schemas"))
		.cloned()
		.unwrap_or(Value::Null);

	let paths = doc.get("paths").cloned().unwrap_or(Value::Null);

	let surface = json!({
		"paths": paths,
		"schemas": schemas,
	});

	assert_or_update_golden("v1_openapi_surface.json", &canonical(&surface));
}

/// Golden JSON snapshots for a representative set of v1 response payloads. These
/// pin the serialized wire shape of shared DTOs independent of the OpenAPI
/// schema derive.
#[test]
fn v1_representative_response_shapes_are_frozen() {
	use service::model::{
		FailedMatchReason, GameMatchType, GameMetadataMatchResult, GameNameSearchResult,
		MetadataProvider,
	};
	use uuid::Uuid;

	let nil = Uuid::nil();

	let game_match_no_match = GameMetadataMatchResult {
		game_match_type: GameMatchType::NoMatch,
		id: None,
		external_metadata: vec![],
	};
	let game_match_hit = GameMetadataMatchResult {
		game_match_type: GameMatchType::SHA256,
		id: Some(nil),
		external_metadata: vec![],
	};
	let search_result = GameNameSearchResult {
		id: nil,
		name: "Sample Game".to_string(),
		platform_id: nil,
		platform_name: "Sample Platform".to_string(),
	};

	let game_match_type_variants: Vec<Value> = [
		GameMatchType::SHA256,
		GameMatchType::SHA1,
		GameMatchType::MD5,
		GameMatchType::CRC,
		GameMatchType::FileNameAndSize,
		GameMatchType::NoMatch,
	]
	.iter()
	.map(|v| serde_json::to_value(v).unwrap())
	.collect();
	let metadata_provider_variants: Vec<Value> = [
		MetadataProvider::IGDB,
		MetadataProvider::SteamGridDB,
		MetadataProvider::ScreenScraper,
		MetadataProvider::MobyGames,
		MetadataProvider::LaunchBox,
		MetadataProvider::EmuReady,
		MetadataProvider::OpenVGDB,
		MetadataProvider::RetroAchievements,
		MetadataProvider::TheGamesDB,
		MetadataProvider::Hasheous,
	]
	.iter()
	.map(|v| serde_json::to_value(v).unwrap())
	.collect();
	let failed_match_reason_variants: Vec<Value> = [
		FailedMatchReason::NoDirectMatch,
		FailedMatchReason::TooManyMatches,
		FailedMatchReason::Ambiguous,
		FailedMatchReason::TooManyFiles,
	]
	.iter()
	.map(|v| serde_json::to_value(v).unwrap())
	.collect();

	let samples = json!({
		"game_match_no_match": serde_json::to_value(&game_match_no_match).unwrap(),
		"game_match_hit": serde_json::to_value(&game_match_hit).unwrap(),
		"game_name_search_result": serde_json::to_value(&search_result).unwrap(),
		"game_match_type_variants": game_match_type_variants,
		"metadata_provider_variants": metadata_provider_variants,
		"failed_match_reason_variants": failed_match_reason_variants,
	});

	assert_or_update_golden("v1_response_shapes.json", &canonical(&samples));
}
