//! Routing-level guards for the unversioned discovery document and the always-on
//! lifecycle Link header. These need no database: the public health route and
//! the discovery handler both answer without extractors.

use actix_web::http::header::HeaderValue;
use actix_web::web::Data;
use actix_web::{App, test, web};
use api::routes::versions::{ResolvedDefaultVersion, get_api_versions};
use api::{
	PublicRouteFlags, public_api_governor_config, test_prometheus_metrics, versioned_api_scope,
};
use service::config::versions::ApiVersion;

macro_rules! get {
	($uri:expr) => {
		test::TestRequest::get()
			.uri($uri)
			.peer_addr("203.0.113.7:0".parse().unwrap())
			.to_request()
	};
}

fn discovery_app(
	default: ApiVersion,
) -> App<
	impl actix_web::dev::ServiceFactory<
		actix_web::dev::ServiceRequest,
		Config = (),
		Response = actix_web::dev::ServiceResponse,
		Error = actix_web::Error,
		InitError = (),
	>,
> {
	let gov = public_api_governor_config();
	let prom = test_prometheus_metrics();
	let flags = PublicRouteFlags::default();
	App::new()
		.service(versioned_api_scope(
			"/api/v1",
			ApiVersion::V1,
			flags,
			&gov,
			prom.clone(),
		))
		.service(versioned_api_scope(
			"/api/v2",
			ApiVersion::V2,
			flags,
			&gov,
			prom.clone(),
		))
		.app_data(Data::new(ResolvedDefaultVersion(default)))
		.route("/api/versions", web::get().to(get_api_versions))
		.service(versioned_api_scope(
			"/api",
			default,
			flags,
			&gov,
			prom.clone(),
		))
}

#[actix_web::test]
async fn discovery_endpoint_is_public_and_cacheable() {
	let app = test::init_service(discovery_app(ApiVersion::V1)).await;

	let resp = test::call_service(&app, get!("/api/versions")).await;
	assert!(resp.status().is_success(), "got {}", resp.status());
	assert_eq!(
		resp.headers().get("Cache-Control"),
		Some(&HeaderValue::from_static("public, max-age=3600"))
	);
	assert_eq!(
		resp.headers().get("Access-Control-Allow-Origin"),
		Some(&HeaderValue::from_static("*"))
	);

	let body = test::read_body(resp).await;
	let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
	assert_eq!(json["default"], "v1");
	assert_eq!(json["request_header"], "Playmatch-Api-Version");
	assert!(json["versions"].as_array().unwrap().len() >= 2);
}

#[actix_web::test]
async fn discovery_default_follows_resolved_default() {
	let app = test::init_service(discovery_app(ApiVersion::V2)).await;
	let resp = test::call_service(&app, get!("/api/versions")).await;
	let body = test::read_body(resp).await;
	let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
	assert_eq!(
		json["default"], "v2",
		"the discovery default must mirror the resolved boot default"
	);
}

#[actix_web::test]
async fn discovery_route_is_not_swallowed_by_the_bare_alias() {
	// /api is a prefix of /api/versions; the discovery route registers before the
	// bare alias so it must win.
	let app = test::init_service(discovery_app(ApiVersion::V1)).await;
	let resp = test::call_service(&app, get!("/api/versions")).await;
	let body = test::read_body(resp).await;
	let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
	assert!(
		json.get("versions").is_some(),
		"the discovery handler, not the alias, must answer /api/versions"
	);
}

#[actix_web::test]
async fn every_api_response_carries_the_discovery_link() {
	let app = test::init_service(discovery_app(ApiVersion::V1)).await;
	let resp = test::call_service(&app, get!("/api/v1/health")).await;

	let links: Vec<String> = resp
		.headers()
		.get_all("Link")
		.map(|v| v.to_str().unwrap_or("").to_string())
		.collect();
	let joined = links.join(",");
	assert!(
		joined.contains("/api/versions"),
		"every API response must Link the discovery doc, got {links:?}"
	);
	assert!(
		joined.contains("version-history"),
		"the discovery Link must use the namespaced rel, got {links:?}"
	);
}

#[actix_web::test]
async fn active_version_emits_no_deprecation_or_sunset() {
	let app = test::init_service(discovery_app(ApiVersion::V1)).await;
	let resp = test::call_service(&app, get!("/api/v1/health")).await;
	assert!(
		resp.headers().get("Deprecation").is_none(),
		"deprecation plumbing must stay inert while v1 is active"
	);
	assert!(resp.headers().get("Sunset").is_none());
}
