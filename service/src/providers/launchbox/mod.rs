use entity::sea_orm_active_enums::MetadataProviderEnum;
use reqwest::Client;
use sea_orm::DbConn;
use std::sync::Arc;

pub mod cache;
pub mod import;
pub mod matching;
pub mod model;

pub struct LaunchBoxClient {
	http: Client,
	redis_conn: redis::aio::MultiplexedConnection,
	db_conn: DbConn,
	metadata_url: String,
}

impl LaunchBoxClient {
	pub fn new(
		http: Client,
		redis_conn: redis::aio::MultiplexedConnection,
		db_conn: DbConn,
		metadata_url: Option<String>,
	) -> anyhow::Result<Self> {
		Ok(Self {
			http,
			redis_conn,
			db_conn,
			metadata_url: metadata_url.unwrap_or_else(|| import::DEFAULT_METADATA_URL.to_string()),
		})
	}

	pub fn redis_conn(&self) -> &redis::aio::MultiplexedConnection {
		&self.redis_conn
	}

	pub async fn ensure_imported(&self) -> anyhow::Result<import::ImportOutcome> {
		import::ensure_imported(&self.http, &self.db_conn, &self.metadata_url).await
	}
}

#[async_trait::async_trait]
impl crate::providers::MetadataProvider for LaunchBoxClient {
	fn provider_label(&self) -> &'static str {
		"launchbox"
	}

	fn provider_enum(&self) -> MetadataProviderEnum {
		MetadataProviderEnum::Launchbox
	}

	async fn match_db(self: Arc<Self>, db_conn: &DbConn) -> anyhow::Result<()> {
		matching::match_db_to_launchbox_entities(self, db_conn).await
	}

	async fn match_via_sibling_names(self: Arc<Self>, db_conn: &DbConn) -> anyhow::Result<()> {
		crate::providers::drive_cross_match_pipeline(
			"launchbox",
			MetadataProviderEnum::Launchbox,
			matching::game::match_game_via_sibling_name_launchbox,
			self,
			db_conn,
			crate::providers::DEFAULT_CHUNK_SIZE,
		)
		.await
	}
}
