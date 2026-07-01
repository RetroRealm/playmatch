//! Guards for the per-version OpenAPI documents and their server URLs. Removing
//! the per-handler context_path means the documented path now comes from the
//! document's server URL; these assert each document renders the reachable URL.

use serde_json::Value;

fn server_urls(doc: &Value) -> Vec<String> {
	doc.get("servers")
		.and_then(|s| s.as_array())
		.map(|arr| {
			arr.iter()
				.filter_map(|s| s.get("url").and_then(|u| u.as_str()).map(str::to_string))
				.collect()
		})
		.unwrap_or_default()
}

fn path_keys(doc: &Value) -> Vec<String> {
	doc.get("paths")
		.and_then(|p| p.as_object())
		.map(|m| m.keys().cloned().collect())
		.unwrap_or_default()
}

#[test]
fn v1_document_serves_under_api_v1_with_relative_paths() {
	let doc: Value = serde_json::to_value(api::create_openapi_v1()).unwrap();

	assert_eq!(
		server_urls(&doc),
		vec!["/api/v1".to_string()],
		"the v1 document must advertise the /api/v1 server"
	);

	let paths = path_keys(&doc);
	assert!(
		paths.iter().any(|p| p == "/game/{id}"),
		"v1 paths are relative so the server URL supplies the prefix"
	);
	assert!(
		paths.iter().all(|p| !p.starts_with("/api/")),
		"no v1 path may carry a hardcoded /api prefix any more"
	);
	assert!(
		paths.iter().all(|p| !p.starts_with("/api/v2")),
		"the v1 document must not leak v2 paths"
	);
}

#[test]
fn v2_document_serves_under_api_v2_with_relative_paths() {
	let doc: Value = serde_json::to_value(api::create_openapi_v2()).unwrap();

	assert_eq!(
		server_urls(&doc),
		vec!["/api/v2".to_string()],
		"the v2 document must advertise the /api/v2 server"
	);

	let paths = path_keys(&doc);
	assert!(
		paths.iter().any(|p| p == "/games"),
		"v2 list path is relative under the /api/v2 server"
	);
	assert!(
		paths.iter().all(|p| !p.starts_with("/api/")),
		"no v2 path may carry a hardcoded prefix"
	);
}

#[test]
fn both_documents_carry_the_bearer_security_scheme() {
	for doc_value in [
		serde_json::to_value(api::create_openapi_v1()).unwrap(),
		serde_json::to_value(api::create_openapi_v2()).unwrap(),
	] {
		let scheme = doc_value
			.get("components")
			.and_then(|c| c.get("securitySchemes"))
			.and_then(|s| s.get("bearer_auth"));
		assert!(
			scheme.is_some(),
			"build_openapi must inject bearer_auth into every document"
		);
	}
}

#[test]
fn transitional_alias_matches_the_v1_document() {
	let alias: Value = serde_json::to_value(api::create_openapi()).unwrap();
	let v1: Value = serde_json::to_value(api::create_openapi_v1()).unwrap();
	assert_eq!(
		alias, v1,
		"the /api-docs/openapi.json alias must mirror the v1 document"
	);
}

fn collect_schema_refs(value: &Value, out: &mut Vec<String>) {
	match value {
		Value::Object(map) => {
			for (key, val) in map {
				if key == "$ref" {
					if let Some(name) = val
						.as_str()
						.and_then(|s| s.strip_prefix("#/components/schemas/"))
					{
						out.push(name.to_string());
					}
				} else {
					collect_schema_refs(val, out);
				}
			}
		}
		Value::Array(items) => items.iter().for_each(|v| collect_schema_refs(v, out)),
		_ => {}
	}
}

fn defined_schema_names(doc: &Value) -> std::collections::BTreeSet<String> {
	doc.get("components")
		.and_then(|c| c.get("schemas"))
		.and_then(|s| s.as_object())
		.map(|m| m.keys().cloned().collect())
		.unwrap_or_default()
}

// A generic alias written as `Page<service::model::Foo>` makes utoipa emit a
// dotted `$ref` (service.model.Foo) that no component defines, which renders an
// unusable Swagger page. Guard that every component reference in both documents
// resolves to a defined schema.
#[test]
fn every_component_ref_resolves_in_both_documents() {
	for (name, doc) in [
		(
			"v1",
			serde_json::to_value(api::create_openapi_v1()).unwrap(),
		),
		(
			"v2",
			serde_json::to_value(api::create_openapi_v2()).unwrap(),
		),
	] {
		let defined = defined_schema_names(&doc);
		let mut refs = Vec::new();
		collect_schema_refs(&doc, &mut refs);
		refs.sort();
		refs.dedup();

		let unresolved: Vec<&String> = refs.iter().filter(|r| !defined.contains(*r)).collect();
		assert!(
			unresolved.is_empty(),
			"{name} document has unresolved component refs: {unresolved:?}"
		);
	}
}

// A schema defined in components but never referenced by any path or other schema
// is dead weight: a codegen client emits an unused model class, and it is how v2
// bulk schemas silently leaked into the frozen v1 document. Guard against it in
// both directions (this is the inverse of every_component_ref_resolves).
//
// utoipa's `IntoParams` inlines query-parameter structs rather than emitting a
// `$ref`, so a handful of parameter-only schemas are legitimately defined without
// being referenced. They are the sole allowed exception.
#[test]
fn no_orphaned_component_schemas_in_either_document() {
	// The identify search and the SteamGridDB asset filter are inlined query
	// parameters; utoipa emits their (and their enum fields') schemas without a
	// `$ref`. Provider responses use plain strings, so these enums appear only as
	// documented filter parameters.
	const PARAM_ONLY_SCHEMAS: &[&str] = &[
		"GameFileMatchSearch",
		"SgdbAssetFilterQuery",
		"AssetFilters",
		"SgdbAssetMime",
		"SgdbAssetType",
		"SgdbContentTag",
		"SgdbGridDimension",
		"SgdbGridStyle",
		"SgdbHeroDimension",
		"SgdbHeroStyle",
		"SgdbIconStyle",
		"SgdbLogoStyle",
		"SgdbTriState",
	];

	for (name, doc) in [
		(
			"v1",
			serde_json::to_value(api::create_openapi_v1()).unwrap(),
		),
		(
			"v2",
			serde_json::to_value(api::create_openapi_v2()).unwrap(),
		),
	] {
		let defined = defined_schema_names(&doc);
		let mut refs = Vec::new();
		collect_schema_refs(&doc, &mut refs);
		let referenced: std::collections::BTreeSet<String> = refs.into_iter().collect();

		let orphans: Vec<&String> = defined
			.iter()
			.filter(|s| !referenced.contains(*s))
			.filter(|s| !PARAM_ONLY_SCHEMAS.contains(&s.as_str()))
			.collect();

		assert!(
			orphans.is_empty(),
			"{name} document defines schemas nothing references: {orphans:?}. Remove them from \
			 components(schemas(...)), or add them to PARAM_ONLY_SCHEMAS if they are inlined query \
			 parameters."
		);
	}
}

fn operation_id(doc: &Value, path: &str, method: &str) -> Option<String> {
	doc.get("paths")?
		.get(path)?
		.get(method)?
		.get("operationId")?
		.as_str()
		.map(str::to_string)
}

// A reused v1 handler mounted in v2 must never be documented under a path the v2
// scope does not actually serve, and must never overwrite the v2 handler that
// owns a shared path. v2 serves the plural game paths and the paginated
// catalogue handlers; the singular /game/... paths and the v1 list handlers must
// not leak into the v2 document.
#[test]
fn v2_document_only_advertises_v2_owned_resources() {
	let doc: Value = serde_json::to_value(api::create_openapi_v2()).unwrap();

	let singular_game: Vec<String> = path_keys(&doc)
		.into_iter()
		.filter(|p| p.starts_with("/game/") || p == "/game")
		.collect();
	assert!(
		singular_game.is_empty(),
		"v2 must use the plural /games paths only, found singular: {singular_game:?}"
	);

	for (path, method, expected) in [
		("/companies", "get", "list_companies_v2"),
		("/companies/{id}", "get", "get_company_by_id_v2"),
		("/platforms", "get", "list_platforms_v2"),
		("/platforms/{id}", "get", "get_platform_by_id_v2"),
		("/signature-groups", "get", "list_signature_groups_v2"),
		(
			"/signature-groups/{id}",
			"get",
			"get_signature_group_by_id_v2",
		),
		("/games/search", "get", "search_games_v2"),
	] {
		assert_eq!(
			operation_id(&doc, path, method).as_deref(),
			Some(expected),
			"v2 {method} {path} must be served by the v2 handler, not a v1 reuse"
		);
	}
}
