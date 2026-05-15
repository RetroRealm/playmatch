use entity::sea_orm_active_enums::MetadataProviderEnum;
use reqwest::Client;
use sea_orm::DbConn;
use std::sync::Arc;

pub mod cache;
pub mod import;
pub mod matching;
pub mod model;

pub struct OpenVgdbClient {
	http: Client,
	redis_conn: redis::aio::MultiplexedConnection,
	db_conn: DbConn,
	metadata_url: String,
}

impl OpenVgdbClient {
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

	pub async fn ensure_imported(&self) -> anyhow::Result<import::ImportOutcome> {
		import::ensure_imported(&self.http, &self.db_conn, &self.metadata_url).await
	}
}

#[async_trait::async_trait]
impl crate::providers::MetadataProvider for OpenVgdbClient {
	fn provider_label(&self) -> &'static str {
		"openvgdb"
	}

	fn provider_enum(&self) -> MetadataProviderEnum {
		MetadataProviderEnum::OpenVGDB
	}

	fn redis_conn(&self) -> &redis::aio::MultiplexedConnection {
		&self.redis_conn
	}

	async fn match_db(self: Arc<Self>, db_conn: &DbConn) -> anyhow::Result<()> {
		matching::match_db_to_openvgdb_entities(self, db_conn).await
	}
}
