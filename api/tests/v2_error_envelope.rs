//! End-to-end guards for the v2 error contract: every v2 4xx answers the
//! machine-readable `{code, message}` envelope, while v1 keeps its frozen
//! plain-text bodies. Drives the real actix version scopes so the error
//! middleware, reused handlers, auth failures and extractor failures are all
//! exercised through the same path a client hits. Require Docker.

use actix_web::web::Data;
use actix_web::{App, test};
use api::{
	PublicRouteFlags, public_api_governor_config, test_prometheus_metrics, versioned_api_scope,
};
use migration::{Migrator, MigratorTrait};
use redis::aio::MultiplexedConnection;
use sea_orm::{Database, DatabaseConnection};
use serde_json::Value;
use service::config::versions::ApiVersion;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::redis::{REDIS_PORT, Redis};
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use testcontainers_modules::testcontainers::{ContainerAsync, ImageExt};

async fn start_pg() -> (ContainerAsync<Postgres>, DatabaseConnection) {
	let container = Postgres::default()
		.with_tag("18-alpine")
		.start()
		.await
		.unwrap();
	let port = container.get_host_port_ipv4(5432).await.unwrap();
	let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
	let db = Database::connect(&url).await.unwrap();
	Migrator::up(&db, None).await.unwrap();
	(container, db)
}

async fn start_redis() -> (ContainerAsync<Redis>, MultiplexedConnection) {
	let container = Redis::default().with_tag("7-alpine").start().await.unwrap();
	let port = container.get_host_port_ipv4(REDIS_PORT).await.unwrap();
	let client = redis::Client::open(format!("redis://127.0.0.1:{port}")).unwrap();
	let conn = client.get_multiplexed_async_connection().await.unwrap();
	(container, conn)
}

macro_rules! build_app {
	($db:expr, $redis:expr) => {{
		let gov = public_api_governor_config();
		App::new()
			.app_data(Data::new($db))
			.app_data(Data::new($redis))
			.service(versioned_api_scope(
				"/api/v2",
				ApiVersion::V2,
				PublicRouteFlags::default(),
				&gov,
				test_prometheus_metrics(),
			))
			.service(versioned_api_scope(
				"/api/v1",
				ApiVersion::V1,
				PublicRouteFlags::default(),
				&gov,
				test_prometheus_metrics(),
			))
	}};
}

/// A GET with a fixed peer address so the rate-limiter key extractor always has
/// an IP to fall back on regardless of proxy-trust configuration.
macro_rules! get {
	($app:expr, $uri:expr) => {{
		let req = test::TestRequest::get()
			.uri($uri)
			.peer_addr("203.0.113.5:9000".parse().unwrap())
			.to_request();
		let resp = test::call_service(&$app, req).await;
		let status = resp.status().as_u16();
		let is_json = resp
			.headers()
			.get("content-type")
			.and_then(|v| v.to_str().ok())
			.map(|v| v.starts_with("application/json"))
			.unwrap_or(false);
		let body = test::read_body(resp).await;
		(status, is_json, body)
	}};
}

#[actix_web::test]
async fn v2_missing_auth_answers_json_envelope() {
	let (_pg, db) = start_pg().await;
	let (_redis, redis) = start_redis().await;
	let app = test::init_service(build_app!(db, redis)).await;

	// An authed v2 route with no Authorization header: auth fails before the
	// handler body runs, and the middleware must render it as JSON, not plain text.
	let (status, is_json, body) = get!(app, "/api/v2/user/11111111-1111-1111-1111-111111111111");
	assert_eq!(status, 401);
	assert!(is_json, "a v2 401 must be JSON, got: {:?}", body);
	let json: Value = serde_json::from_slice(&body).unwrap();
	assert_eq!(json["code"], "invalid_auth");
	assert!(json["message"].is_string());
}

#[actix_web::test]
async fn v2_malformed_query_param_answers_json_envelope() {
	let (_pg, db) = start_pg().await;
	let (_redis, redis) = start_redis().await;
	let app = test::init_service(build_app!(db, redis)).await;

	// limit is a u64; a non-numeric value fails the query extractor before the
	// handler runs. The middleware must still answer the v2 envelope.
	let (status, is_json, body) = get!(app, "/api/v2/games?limit=not-a-number");
	assert_eq!(status, 400);
	assert!(is_json, "a v2 extractor 400 must be JSON, got: {:?}", body);
	let json: Value = serde_json::from_slice(&body).unwrap();
	assert!(json["code"].is_string());
	assert!(json["message"].is_string());
}

#[actix_web::test]
async fn v2_malformed_path_answers_json_envelope() {
	let (_pg, db) = start_pg().await;
	let (_redis, redis) = start_redis().await;
	let app = test::init_service(build_app!(db, redis)).await;

	let (_status, is_json, body) = get!(app, "/api/v2/games/not-a-uuid");
	assert!(
		is_json,
		"a v2 malformed-path error must be JSON, got: {:?}",
		body
	);
	let json: Value = serde_json::from_slice(&body).unwrap();
	assert!(json["code"].is_string());
}

#[actix_web::test]
async fn v2_identify_ids_invalid_query_is_json_envelope() {
	let (_pg, db) = start_pg().await;
	let (_redis, redis) = start_redis().await;
	let app = test::init_service(build_app!(db, redis)).await;

	// The v2 /identify/ids fork must answer the JSON envelope with the shared
	// invalid_query code on a malformed crc, matching /identify/relations. The
	// required fileName/fileSize are supplied so the request clears the extractor
	// and reaches the handler's own validation.
	let (status, is_json, body) = get!(
		app,
		"/api/v2/identify/ids?fileName=game.rom&fileSize=1024&crc=zzz"
	);
	assert_eq!(status, 400);
	assert!(is_json, "v2 identify/ids 400 must be JSON, got: {:?}", body);
	let json: Value = serde_json::from_slice(&body).unwrap();
	assert_eq!(json["code"], "invalid_query");
}

#[actix_web::test]
async fn v2_unknown_route_answers_json_envelope() {
	let (_pg, db) = start_pg().await;
	let (_redis, redis) = start_redis().await;
	let app = test::init_service(build_app!(db, redis)).await;

	let (status, is_json, body) = get!(app, "/api/v2/this-route-does-not-exist");
	assert_eq!(status, 404);
	assert!(
		is_json,
		"a v2 unmatched-route 404 must be JSON, got: {:?}",
		body
	);
	let json: Value = serde_json::from_slice(&body).unwrap();
	assert_eq!(json["code"], "not_found");
}

#[actix_web::test]
async fn v2_wrong_method_answers_json_envelope() {
	let (_pg, db) = start_pg().await;
	let (_redis, redis) = start_redis().await;
	let app = test::init_service(build_app!(db, redis)).await;

	// /games is GET-only; a POST must answer the v2 envelope, not an empty body.
	let req = test::TestRequest::post()
		.uri("/api/v2/games")
		.peer_addr("203.0.113.5:9000".parse().unwrap())
		.to_request();
	let resp = test::call_service(&app, req).await;
	let status = resp.status().as_u16();
	let is_json = resp
		.headers()
		.get("content-type")
		.and_then(|v| v.to_str().ok())
		.map(|v| v.starts_with("application/json"))
		.unwrap_or(false);
	let body = test::read_body(resp).await;
	// actix resolves a method mismatch in a scope to 404; either way the body must
	// be the v2 JSON envelope, never an empty/plain-text response.
	assert!(
		status == 404 || status == 405,
		"unexpected status for wrong method: {status}"
	);
	assert!(
		is_json,
		"a v2 wrong-method error must be JSON, got: {:?}",
		body
	);
	let json: Value = serde_json::from_slice(&body).unwrap();
	assert!(
		json["code"] == "not_found" || json["code"] == "method_not_allowed",
		"unexpected code: {}",
		json["code"]
	);
}

#[actix_web::test]
async fn v2_rate_limit_429_answers_json_envelope() {
	let (_pg, db) = start_pg().await;
	let (_redis, redis) = start_redis().await;
	let app = test::init_service(build_app!(db, redis)).await;

	// The governor burst is 20 for one client key. Hammer from a single peer until
	// it throttles, then assert the 429 is the JSON envelope, not the Governor's
	// default plain-text body, and that it kept its Retry-After header.
	let mut throttled = None;
	for _ in 0..40 {
		let req = test::TestRequest::get()
			.uri("/api/v2/companies")
			.peer_addr("198.51.100.77:5000".parse().unwrap())
			.to_request();
		let resp = test::call_service(&app, req).await;
		if resp.status().as_u16() == 429 {
			throttled = Some(resp);
			break;
		}
	}

	let resp = throttled.expect("the governor must eventually return a 429");
	let is_json = resp
		.headers()
		.get("content-type")
		.and_then(|v| v.to_str().ok())
		.map(|v| v.starts_with("application/json"))
		.unwrap_or(false);
	let has_retry_after = resp.headers().contains_key("retry-after");
	let body = test::read_body(resp).await;
	assert!(is_json, "a v2 429 must be JSON, got: {:?}", body);
	assert!(
		has_retry_after,
		"the rewritten 429 must keep its Retry-After header"
	);
	let json: Value = serde_json::from_slice(&body).unwrap();
	assert_eq!(json["code"], "rate_limited");
}

#[actix_web::test]
async fn v1_identify_ids_invalid_query_stays_plain_text() {
	let (_pg, db) = start_pg().await;
	let (_redis, redis) = start_redis().await;
	let app = test::init_service(build_app!(db, redis)).await;

	// v1 is frozen: the same malformed query must keep the v1 plain-text body and
	// never adopt the v2 JSON envelope.
	let (status, is_json, body) = get!(
		app,
		"/api/v1/identify/ids?fileName=game.rom&fileSize=1024&crc=zzz"
	);
	assert_eq!(status, 400);
	assert!(
		!is_json,
		"v1 identify/ids 400 must stay plain text, got JSON: {:?}",
		body
	);
}
