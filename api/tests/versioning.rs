//! Routing-level guards for the path-versioned API. These exercise scope
//! resolution and response headers and need no database: the public health
//! route answers without extractors, and the authenticated sub-scope returns
//! an error before its handler runs while still flowing through both header
//! middlewares.

use actix_web::http::StatusCode;
use actix_web::http::header::HeaderValue;
use actix_web::{App, test};
use api::{
	PublicRouteFlags, public_api_governor_config, test_prometheus_metrics, versioned_api_scope,
};
use service::config::versions::ApiVersion;

fn build_app_config() -> (
	actix_governor::GovernorConfig<
		api::ReverProxyExtractor,
		actix_governor::governor::middleware::StateInformationMiddleware,
	>,
	actix_web_prom::PrometheusMetrics,
) {
	(public_api_governor_config(), test_prometheus_metrics())
}

// The rate-limiter key extractor needs a client IP; supply one so Governor
// does not reject the request before it reaches the scope.
macro_rules! get {
	($uri:expr) => {
		test::TestRequest::get()
			.uri($uri)
			.peer_addr("203.0.113.7:0".parse().unwrap())
			.to_request()
	};
}

macro_rules! versioned_app {
	($default:expr) => {{
		let (gov, prom) = build_app_config();
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
			.service(versioned_api_scope(
				"/api",
				$default,
				flags,
				&gov,
				prom.clone(),
			))
	}};
}

#[actix_web::test]
async fn v1_list_route_resolves_to_v1_scope_not_bare_alias() {
	let app = test::init_service(versioned_app!(ApiVersion::V1)).await;

	let req = get!("/api/v1/health");
	let resp = test::call_service(&app, req).await;

	assert!(
		resp.status().is_success(),
		"/api/v1/health must resolve inside the explicit v1 scope, got {}",
		resp.status()
	);
	assert_eq!(
		resp.headers().get("Playmatch-Api-Version"),
		Some(&HeaderValue::from_static("v1"))
	);
}

#[actix_web::test]
async fn bare_alias_serves_default_version_routes_and_header() {
	let app = test::init_service(versioned_app!(ApiVersion::V1)).await;

	let req = get!("/api/health");
	let resp = test::call_service(&app, req).await;

	assert!(
		resp.status().is_success(),
		"/api/health must resolve through the bare alias, got {}",
		resp.status()
	);
	assert_eq!(
		resp.headers().get("Playmatch-Api-Version"),
		Some(&HeaderValue::from_static("v1")),
		"the bare alias must report the resolved default version"
	);
}

#[actix_web::test]
async fn explicit_scopes_register_before_bare_alias() {
	// With the default resolved to v2, /api/health must report v2 while
	// /api/v1/health still reports v1. This proves the bare alias does not
	// swallow the explicit version prefixes.
	let app = test::init_service(versioned_app!(ApiVersion::V2)).await;

	let v1 = test::call_service(&app, get!("/api/v1/health")).await;
	assert!(v1.status().is_success());
	assert_eq!(
		v1.headers().get("Playmatch-Api-Version"),
		Some(&HeaderValue::from_static("v1"))
	);

	let bare = test::call_service(&app, get!("/api/health")).await;
	assert_eq!(
		bare.headers().get("Playmatch-Api-Version"),
		Some(&HeaderValue::from_static("v2")),
		"the bare alias must mirror the resolved default (v2 here)"
	);
}

#[actix_web::test]
async fn version_header_does_not_replace_build_version_header() {
	let app = test::init_service(versioned_app!(ApiVersion::V1)).await;

	let req = get!("/api/v1/health");
	let resp = test::call_service(&app, req).await;

	assert_eq!(
		resp.headers().get("X-Version"),
		Some(&HeaderValue::from_static(env!("CARGO_PKG_VERSION"))),
		"the build SemVer header must remain byte-for-byte on versioned paths"
	);
	assert!(
		resp.headers().get("Content-Location").is_none(),
		"version aliasing must not emit Content-Location"
	);
}

#[actix_web::test]
async fn authenticated_responses_carry_both_vary_values() {
	let app = test::init_service(versioned_app!(ApiVersion::V1)).await;

	// An authenticated route with no app data registered fails extraction and
	// returns an error response, which still flows through both DefaultHeaders
	// layers (outer Vary: Origin, inner Vary: Authorization).
	let req = get!("/api/v1/user/00000000-0000-0000-0000-000000000000");
	let resp = test::call_service(&app, req).await;

	let vary_values: Vec<String> = resp
		.headers()
		.get_all("Vary")
		.map(|v| v.to_str().unwrap_or("").to_ascii_lowercase())
		.collect();
	let joined = vary_values.join(",");

	assert!(
		joined.contains("origin"),
		"authenticated response must carry Vary: Origin, got {vary_values:?}"
	);
	assert!(
		joined.contains("authorization"),
		"authenticated response must carry Vary: Authorization, got {vary_values:?}"
	);
}

#[actix_web::test]
async fn v2_routes_resolve_inside_the_v2_scope() {
	// Regression guard for the empty-scope swallow bug: a leading `scope("")`
	// registered ahead of the other v2 services used to capture every request and
	// 404 it. With no app data these handlers fail extraction and return an error,
	// but a registered route never returns 404, so a non-404 status proves the
	// route is reachable. Covers a list, a literal sub-path, a dynamic id, a bulk
	// POST target and the authenticated route, one per former empty scope.
	let app = test::init_service(versioned_app!(ApiVersion::V1)).await;

	for uri in [
		"/api/v2/companies",
		"/api/v2/platforms",
		"/api/v2/stats",
		"/api/v2/games",
		"/api/v2/games/search?query=mario",
		"/api/v2/dat-files",
		"/api/v2/dat-files/by-hash?sha256=deadbeef",
		"/api/v2/games/00000000-0000-0000-0000-000000000000",
		"/api/v2/suggestion",
	] {
		let resp = test::call_service(&app, get!(uri)).await;
		assert_ne!(
			resp.status(),
			StatusCode::NOT_FOUND,
			"{uri} must resolve inside the v2 scope, got {}",
			resp.status()
		);
	}
}

#[actix_web::test]
async fn v2_has_full_functional_parity_with_v1() {
	// v2 now carries the reused v1 handlers (single identify, health, game-file
	// history, manual match, suggestion CRUD, users, provider proxies) plus the new
	// entity searches and bulk dat-files. With all provider flags enabled and no app
	// data, each route fails extraction or auth but must never 404, which proves the
	// parity wiring actually mounts every route in the v2 scope.
	let (gov, prom) = build_app_config();
	let flags = PublicRouteFlags {
		igdb_enabled: true,
		sgdb_enabled: true,
		ss_enabled: true,
		mg_enabled: true,
		lb_enabled: true,
		ovgdb_enabled: true,
		ra_enabled: true,
	};
	let app = test::init_service(App::new().service(versioned_api_scope(
		"/api/v2",
		ApiVersion::V2,
		flags,
		&gov,
		prom,
	)))
	.await;

	let id = "00000000-0000-0000-0000-000000000000";
	let gets = [
		"/api/v2/companies/search?query=mario".to_string(),
		"/api/v2/platforms/search?query=snes".to_string(),
		"/api/v2/signature-groups/search?query=no-intro".to_string(),
		"/api/v2/identify/ids?sha256=00".to_string(),
		"/api/v2/identify/relations?sha256=00".to_string(),
		"/api/v2/health".to_string(),
		format!("/api/v2/game-files/{id}/history"),
		format!("/api/v2/suggestion/{id}"),
		format!("/api/v2/user/{id}"),
		"/api/v2/igdb/game?id=1".to_string(),
		"/api/v2/sgdb/game?id=1".to_string(),
		"/api/v2/launchbox/platforms".to_string(),
		"/api/v2/openvgdb/rom/by-hash?sha1=00".to_string(),
	];
	for uri in gets {
		let resp = test::call_service(&app, get!(uri.as_str())).await;
		assert_ne!(
			resp.status(),
			StatusCode::NOT_FOUND,
			"GET {uri} must resolve in the v2 scope, got {}",
			resp.status()
		);
	}

	for uri in [
		"/api/v2/match/manual/game",
		"/api/v2/suggestion/game",
		"/api/v2/dat-files/bulk",
	] {
		let req = test::TestRequest::post()
			.uri(uri)
			.peer_addr("203.0.113.7:0".parse().unwrap())
			.to_request();
		let resp = test::call_service(&app, req).await;
		assert_ne!(
			resp.status(),
			StatusCode::NOT_FOUND,
			"POST {uri} must resolve in the v2 scope, got {}",
			resp.status()
		);
	}
}
