use entity::sea_orm_active_enums::MetadataProviderEnum;
use reqwest::Client;
use sea_orm::DbConn;
use std::sync::Arc;

pub mod api;
pub mod cache;
pub mod import;
pub mod matching;
pub mod model;

pub const API_BASE: &str = "https://retroachievements.org/API";

pub struct RetroAchievementsClient {
	http: Client,
	redis_conn: redis::aio::MultiplexedConnection,
	db_conn: DbConn,
	username: String,
	api_key: String,
	api_base: String,
}

impl RetroAchievementsClient {
	pub fn new(
		username: String,
		api_key: String,
		http: Client,
		redis_conn: redis::aio::MultiplexedConnection,
		db_conn: DbConn,
	) -> anyhow::Result<Self> {
		Ok(Self {
			http,
			redis_conn,
			db_conn,
			username,
			api_key,
			api_base: API_BASE.to_string(),
		})
	}

	pub async fn ensure_imported(&self) -> anyhow::Result<import::ImportOutcome> {
		import::ensure_imported(self).await
	}

	pub(crate) fn http(&self) -> &Client {
		&self.http
	}

	pub(crate) fn api_base(&self) -> &str {
		&self.api_base
	}

	pub(crate) fn username(&self) -> &str {
		&self.username
	}

	pub(crate) fn api_key(&self) -> &str {
		&self.api_key
	}

	pub(crate) fn db_conn(&self) -> &DbConn {
		&self.db_conn
	}
}

#[async_trait::async_trait]
impl crate::providers::MetadataProvider for RetroAchievementsClient {
	fn provider_label(&self) -> &'static str {
		"retroachievements"
	}

	fn provider_enum(&self) -> MetadataProviderEnum {
		MetadataProviderEnum::RetroAchievements
	}

	fn redis_conn(&self) -> &redis::aio::MultiplexedConnection {
		&self.redis_conn
	}

	async fn match_db(self: Arc<Self>, db_conn: &DbConn) -> anyhow::Result<()> {
		matching::match_db_to_retroachievements_entities(self, db_conn).await
	}
}
