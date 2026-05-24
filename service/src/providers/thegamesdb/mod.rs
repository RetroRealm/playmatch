use crate::http::abstraction::RetryPolicy;
use entity::sea_orm_active_enums::MetadataProviderEnum;
use reqwest::{Client, Request, Response};
use sea_orm::DbConn;
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, AtomicU32, Ordering};
use tokio::sync::Mutex;
use tower::retry::Retry;
use tower::{Service, ServiceBuilder, ServiceExt};

pub mod api;
pub mod matching;
pub mod model;

/// Hard cap on TGDB API search calls per match cycle. Combined with the daily
/// cron and 1000-request-per-month free tier, this keeps monthly usage under
/// the 1000 cap with a comfortable buffer.
const PER_CYCLE_CAP_DEFAULT: u32 = 30;
const REMAINING_CACHE_TTL_SECS_DEFAULT: u64 = 24 * 60 * 60;
const REMAINING_CACHE_KEY_DEFAULT: &str = "playmatch:tgdb:remaining_monthly_allowance";

pub struct TheGamesDbClient {
	http: Client,
	service: Mutex<Retry<RetryPolicy, Client>>,
	redis_conn: redis::aio::MultiplexedConnection,
	db_conn: DbConn,
	api_key: Option<String>,
	per_cycle_calls: AtomicU32,
	/// Mirror of TheGamesDB's `remaining_monthly_allowance` for the current
	/// account. `i32::MIN` means "unknown, fetch from Redis or probe".
	remaining_allowance: AtomicI32,
}

impl TheGamesDbClient {
	pub const PER_CYCLE_CAP: u32 = PER_CYCLE_CAP_DEFAULT;
	pub const REMAINING_CACHE_TTL_SECS: u64 = REMAINING_CACHE_TTL_SECS_DEFAULT;
	pub const REMAINING_CACHE_KEY: &'static str = REMAINING_CACHE_KEY_DEFAULT;

	pub fn new(
		http: Client,
		redis_conn: redis::aio::MultiplexedConnection,
		db_conn: DbConn,
		api_key: Option<String>,
	) -> anyhow::Result<Self> {
		let retry_layer = tower::retry::RetryLayer::new(RetryPolicy::new("thegamesdb"));
		let service = ServiceBuilder::new()
			.layer(retry_layer)
			.service(http.clone());

		Ok(Self {
			http,
			service: Mutex::new(service),
			redis_conn,
			db_conn,
			api_key,
			per_cycle_calls: AtomicU32::new(0),
			remaining_allowance: AtomicI32::new(i32::MIN),
		})
	}

	pub(crate) async fn execute(&self, req: Request) -> reqwest::Result<Response> {
		let mut svc = self.service.lock().await;
		let svc = svc.ready().await?;
		svc.call(req).await
	}

	pub fn db_conn(&self) -> &DbConn {
		&self.db_conn
	}

	pub fn has_api_key(&self) -> bool {
		self.api_key.is_some()
	}

	pub(crate) fn reset_cycle_counter(&self) {
		self.per_cycle_calls.store(0, Ordering::Relaxed);
	}
}

#[async_trait::async_trait]
impl crate::providers::MetadataProvider for TheGamesDbClient {
	fn provider_label(&self) -> &'static str {
		"thegamesdb"
	}

	fn provider_enum(&self) -> MetadataProviderEnum {
		MetadataProviderEnum::TheGamesDB
	}

	fn redis_conn(&self) -> &redis::aio::MultiplexedConnection {
		&self.redis_conn
	}

	async fn match_db(self: Arc<Self>, db_conn: &DbConn) -> anyhow::Result<()> {
		self.reset_cycle_counter();
		matching::match_db_to_thegamesdb_entities(self, db_conn).await
	}

	async fn match_via_sibling_names(self: Arc<Self>, db_conn: &DbConn) -> anyhow::Result<()> {
		crate::providers::drive_cross_match_pipeline(
			"thegamesdb",
			MetadataProviderEnum::TheGamesDB,
			matching::game::match_game_via_sibling_name_thegamesdb,
			self,
			db_conn,
			crate::providers::DEFAULT_CHUNK_SIZE,
		)
		.await
	}
}
