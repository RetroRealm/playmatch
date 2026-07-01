//! Guards the v2 OpenAPI document against snake_case leaking into playmatch-owned
//! schemas and query parameters. Provider passthrough schemas mirror their upstream
//! wire shapes, so they are excluded by an explicit allowlist enumerated from the
//! provider model modules registered in `api/src/openapi/v2.rs`. An allowlist, not a
//! name-prefix heuristic, so a future playmatch-owned type can never be silently
//! skipped. No Docker needed.

use serde_json::Value;
use std::collections::BTreeSet;

/// Every provider passthrough schema registered in the v2 document. These mirror an
/// upstream API's wire shape, so snake_case in their properties is expected and not
/// a defect. Enumerated by hand from the `service::providers::*::model` imports in
/// `api/src/openapi/v2.rs`; keep it in sync when a provider schema is added or
/// removed. `SgdbAssetFilterQuery` is deliberately absent: it is a playmatch-owned
/// wrapper (`crate::model::sgdb`), not a provider model, so it stays under the check.
const PROVIDER_SCHEMAS: &[&str] = &[
	// IGDB (service::providers::igdb::model)
	"AgeRating",
	"AgeRatingCategory",
	"AgeRatingContentDescriptionType",
	"AgeRatingContentDescriptionV2",
	"AgeRatingOrganization",
	"AlternativeName",
	"Artwork",
	"ArtworkType",
	"Character",
	"CharacterGender",
	"CharacterMugShot",
	"CharacterSpecies",
	"Collection",
	"CollectionMembership",
	"CollectionMembershipType",
	"CollectionRelation",
	"CollectionRelationType",
	"CollectionType",
	"Company",
	"CompanyLogo",
	"CompanySize",
	"CompanyStatus",
	"CompanyType",
	"CompanyTypeHistory",
	"CompanyWebsite",
	"Cover",
	"DateFormat",
	"EntityType",
	"Event",
	"EventLogo",
	"EventNetwork",
	"ExternalGame",
	"ExternalGameSource",
	"Franchise",
	"Game",
	"GameEngine",
	"GameEngineLogo",
	"GameLocalization",
	"GameMode",
	"GameReleaseFormat",
	"GameStatus",
	"GameTimeToBeat",
	"GameType",
	"GameVersion",
	"GameVersionFeature",
	"GameVersionFeatureCategory",
	"GameVersionFeatureValue",
	"GameVersionFeatureValueEnum",
	"GameVideo",
	"Genre",
	"InvolvedCompany",
	"Keyword",
	"Language",
	"LanguageSupport",
	"LanguageSupportType",
	"MultiplayerMode",
	"NetworkType",
	"Platform",
	"PlatformFamily",
	"PlatformLogo",
	"PlatformType",
	"PlatformVersion",
	"PlatformVersionCompany",
	"PlatformVersionReleaseDate",
	"PlatformWebsite",
	"PlayerPerspective",
	"PopularityPrimitive",
	"PopularityType",
	"Region",
	"ReleaseDate",
	"ReleaseDateRegion",
	"ReleaseDateStatus",
	"Report",
	"ReportType",
	"Screenshot",
	"Theme",
	"Website",
	"WebsiteType",
	// LaunchBox (service::providers::launchbox::model)
	"LbGame",
	"LbGameAlternateName",
	"LbGameImage",
	"LbPlatform",
	// MobyGames (service::providers::mobygames::model)
	"MgAltTitle",
	"MgCover",
	"MgCoverGroup",
	"MgCoversResp",
	"MgGame",
	"MgGamePlatformBrief",
	"MgGenre",
	"MgPlatform",
	"MgSampleCover",
	"MgSampleScreenshot",
	"MgScreenshot",
	"MgScreenshotsResp",
	// OpenVGDB (service::providers::openvgdb::model)
	"OvgdbRelease",
	"OvgdbRom",
	"OvgdbRomMatch",
	// RetroAchievements (service::providers::retroachievements::model)
	"RaGame",
	"RaGameHash",
	"RaGameMatch",
	"RaSystem",
	// ScreenScraper (service::providers::screenscraper::model)
	"SsEntityRef",
	"SsGame",
	"SsLocalizedName",
	"SsRom",
	"SsSystem",
	"SsSystemNames",
	// SteamGridDB (service::providers::steamgriddb::model)
	"AssetFilters",
	"SgdbAsset",
	"SgdbAssetMime",
	"SgdbAssetType",
	"SgdbAuthor",
	"SgdbContentTag",
	"SgdbGame",
	"SgdbGridDimension",
	"SgdbGridStyle",
	"SgdbHeroDimension",
	"SgdbHeroStyle",
	"SgdbIconStyle",
	"SgdbLogoStyle",
	"SgdbPlatform",
	"SgdbTriState",
];

/// First path segment of every provider proxy route group. Their query parameters
/// mirror the upstream API and may carry snake_case, so the path-parameter guard
/// skips them.
const PROVIDER_PATH_PREFIXES: &[&str] = &[
	"/igdb",
	"/launchbox",
	"/mobygames",
	"/openvgdb",
	"/retroachievements",
	"/screenscraper",
	"/sgdb",
];

fn is_provider_schema(schema_name: &str) -> bool {
	PROVIDER_SCHEMAS.contains(&schema_name)
}

fn is_provider_path(path: &str) -> bool {
	PROVIDER_PATH_PREFIXES
		.iter()
		.any(|prefix| path == *prefix || path.starts_with(&format!("{prefix}/")))
}

fn collect_property_names(node: &Value, out: &mut BTreeSet<String>) {
	if let Value::Object(map) = node {
		if let Some(Value::Object(props)) = map.get("properties") {
			for key in props.keys() {
				out.insert(key.clone());
			}
			for value in props.values() {
				collect_property_names(value, out);
			}
		}
		if let Some(items) = map.get("items") {
			collect_property_names(items, out);
		}
		if let Some(additional) = map.get("additionalProperties") {
			collect_property_names(additional, out);
		}
		for combinator in ["allOf", "anyOf", "oneOf"] {
			if let Some(Value::Array(members)) = map.get(combinator) {
				for member in members {
					collect_property_names(member, out);
				}
			}
		}
	}
}

fn collect_enum_values(node: &Value, out: &mut BTreeSet<String>) {
	if let Value::Object(map) = node {
		if let Some(Value::Array(values)) = map.get("enum") {
			for value in values {
				if let Value::String(s) = value {
					out.insert(s.clone());
				}
			}
		}
		if let Some(Value::Object(props)) = map.get("properties") {
			for value in props.values() {
				collect_enum_values(value, out);
			}
		}
		if let Some(items) = map.get("items") {
			collect_enum_values(items, out);
		}
		if let Some(additional) = map.get("additionalProperties") {
			collect_enum_values(additional, out);
		}
		for combinator in ["allOf", "anyOf", "oneOf"] {
			if let Some(Value::Array(members)) = map.get(combinator) {
				for member in members {
					collect_enum_values(member, out);
				}
			}
		}
	}
}

fn openapi_value() -> Value {
	serde_json::to_value(api::create_openapi_v2()).expect("v2 openapi serializes to json")
}

fn schemas(doc: &Value) -> &serde_json::Map<String, Value> {
	doc.get("components")
		.and_then(|c| c.get("schemas"))
		.and_then(Value::as_object)
		.expect("components.schemas present in v2 openapi")
}

#[test]
fn non_provider_schemas_are_camel_case() {
	let doc = openapi_value();
	let schemas = schemas(&doc);

	let mut offenders: Vec<(String, Vec<String>)> = Vec::new();

	for (name, schema) in schemas {
		if is_provider_schema(name) {
			continue;
		}

		let mut props = BTreeSet::new();
		collect_property_names(schema, &mut props);

		let snake: Vec<String> = props.into_iter().filter(|p| p.contains('_')).collect();
		if !snake.is_empty() {
			offenders.push((name.clone(), snake));
		}
	}

	assert!(
		offenders.is_empty(),
		"non-provider v2 schemas must expose only camelCase properties, found snake_case in:\n{}",
		offenders
			.iter()
			.map(|(name, props)| format!("  {name}: {}", props.join(", ")))
			.collect::<Vec<_>>()
			.join("\n")
	);
}

#[test]
fn non_provider_schema_enum_values_are_camel_case() {
	let doc = openapi_value();
	let schemas = schemas(&doc);

	let mut offenders: Vec<(String, Vec<String>)> = Vec::new();

	for (name, schema) in schemas {
		if is_provider_schema(name) {
			continue;
		}

		let mut values = BTreeSet::new();
		collect_enum_values(schema, &mut values);

		let snake: Vec<String> = values.into_iter().filter(|v| v.contains('_')).collect();
		if !snake.is_empty() {
			offenders.push((name.clone(), snake));
		}
	}

	assert!(
		offenders.is_empty(),
		"non-provider v2 schema enum values must be camelCase, found snake_case in:\n{}",
		offenders
			.iter()
			.map(|(name, vals)| format!("  {name}: {}", vals.join(", ")))
			.collect::<Vec<_>>()
			.join("\n")
	);
}

#[test]
fn non_provider_path_query_parameters_are_camel_case() {
	let doc = openapi_value();
	let paths = doc
		.get("paths")
		.and_then(Value::as_object)
		.expect("paths present in v2 openapi");

	let mut offenders: Vec<(String, String, Vec<String>)> = Vec::new();

	for (path, item) in paths {
		if is_provider_path(path) {
			continue;
		}
		let Some(methods) = item.as_object() else {
			continue;
		};
		for (method, operation) in methods {
			let Some(params) = operation.get("parameters").and_then(Value::as_array) else {
				continue;
			};
			let snake: Vec<String> = params
				.iter()
				.filter(|param| {
					matches!(
						param.get("in").and_then(Value::as_str),
						Some("query") | Some("path")
					)
				})
				.filter_map(|param| param.get("name").and_then(Value::as_str))
				.filter(|name| name.contains('_'))
				.map(str::to_string)
				.collect();
			if !snake.is_empty() {
				offenders.push((path.clone(), method.clone(), snake));
			}
		}
	}

	assert!(
		offenders.is_empty(),
		"non-provider v2 paths must expose only camelCase query and path parameters, found snake_case in:\n{}",
		offenders
			.iter()
			.map(|(path, method, params)| format!("  {method} {path}: {}", params.join(", ")))
			.collect::<Vec<_>>()
			.join("\n")
	);
}

#[test]
fn core_v2_schemas_present_and_named() {
	let doc = openapi_value();
	let schemas = schemas(&doc);

	let required = [
		"PlaymatchGameV2",
		"PlaymatchGameFileV2",
		"PlaymatchDatFileV2",
		"PlaymatchSignatureGroupV2",
		"GameNameSearchResultV2",
	];

	let missing: Vec<&str> = required
		.iter()
		.copied()
		.filter(|name| !schemas.contains_key(*name))
		.collect();

	assert!(
		missing.is_empty(),
		"expected core v2 schemas to be present in components.schemas, missing: {missing:?}"
	);
}

#[test]
fn provider_allowlist_only_names_registered_schemas() {
	let doc = openapi_value();
	let schemas = schemas(&doc);

	let stale: Vec<&str> = PROVIDER_SCHEMAS
		.iter()
		.copied()
		.filter(|name| !schemas.contains_key(*name))
		.collect();

	assert!(
		stale.is_empty(),
		"the provider allowlist names schemas absent from the v2 document; remove the stale entries: {stale:?}"
	);
}
