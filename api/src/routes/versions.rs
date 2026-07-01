use actix_web::HttpResponse;
use actix_web::web::Data;
use serde::Serialize;
use service::config::versions::{API_VERSIONS, ApiVersion, latest_version};

/// The header a client sends to pin a specific version. Path-segment versioning
/// is authoritative; this names the optional negotiation header for clients that
/// prefer header-driven selection at a proxy layer.
const VERSION_REQUEST_HEADER: &str = "Playmatch-Api-Version";

#[derive(Debug, Serialize)]
struct VersionEntry {
	version: &'static str,
	status: &'static str,
	is_default: bool,
	released_on: Option<&'static str>,
	deprecated_on: Option<&'static str>,
	sunset_on: Option<&'static str>,
	removed_on: Option<&'static str>,
	docs_url: Option<&'static str>,
	openapi_url: Option<&'static str>,
}

#[derive(Debug, Serialize)]
struct VersionDiscovery {
	default: &'static str,
	latest: &'static str,
	request_header: &'static str,
	versions: Vec<VersionEntry>,
}

/// Resolved default version, injected once at boot so the discovery document and
/// the bare `/api` alias agree on what "default" means for this process.
#[derive(Clone, Copy)]
pub struct ResolvedDefaultVersion(pub ApiVersion);

fn build_discovery(default: ApiVersion) -> VersionDiscovery {
	let versions = API_VERSIONS
		.iter()
		.map(|info| VersionEntry {
			version: info.version.as_str(),
			status: info.status.as_str(),
			is_default: info.version == default,
			released_on: info.released_on,
			deprecated_on: info.deprecated_on,
			sunset_on: info.sunset_on,
			removed_on: info.removed_on,
			docs_url: info.docs_url,
			openapi_url: info.openapi_url,
		})
		.collect();

	VersionDiscovery {
		default: default.as_str(),
		latest: latest_version().as_str(),
		request_header: VERSION_REQUEST_HEADER,
		versions,
	}
}

/// Public, cacheable, unversioned discovery document. Lives outside every
/// version scope and is treated like `.well-known/mcp.json`: permissive CORS and
/// a one hour cache.
pub async fn get_api_versions(default: Data<ResolvedDefaultVersion>) -> HttpResponse {
	let body = build_discovery(default.0);
	HttpResponse::Ok()
		.insert_header(("Access-Control-Allow-Origin", "*"))
		.insert_header(("Access-Control-Allow-Methods", "GET"))
		.insert_header(("Access-Control-Allow-Headers", "Content-Type"))
		.insert_header(("Cache-Control", "public, max-age=3600"))
		.insert_header(("X-Content-Type-Options", "nosniff"))
		.json(body)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn discovery_marks_resolved_default_and_reports_latest() {
		let doc = build_discovery(ApiVersion::V2);
		assert_eq!(doc.default, "v2");
		assert_eq!(doc.request_header, VERSION_REQUEST_HEADER);

		let v2 = doc
			.versions
			.iter()
			.find(|v| v.version == "v2")
			.expect("v2 present");
		assert!(v2.is_default, "the resolved default must be flagged");

		let v1 = doc
			.versions
			.iter()
			.find(|v| v.version == "v1")
			.expect("v1 present");
		assert!(!v1.is_default, "only the resolved default is flagged");

		assert_eq!(
			doc.versions.len(),
			API_VERSIONS.len(),
			"every registry entry is surfaced"
		);
	}

	#[test]
	fn discovery_default_follows_the_argument_not_the_registry_flag() {
		let doc = build_discovery(ApiVersion::V1);
		assert_eq!(doc.default, "v1");
		assert!(
			doc.versions
				.iter()
				.find(|v| v.version == "v1")
				.unwrap()
				.is_default
		);
		assert!(
			!doc.versions
				.iter()
				.find(|v| v.version == "v2")
				.unwrap()
				.is_default
		);
	}
}
