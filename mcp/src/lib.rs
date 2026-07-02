//! MCP server crate: read-only tools over the service layer, packaged as a
//! streamable HTTP service that the api crate mounts into its actix app at
//! `/mcp`. Also builds the JSON for the `.well-known` discovery card.

mod card;
mod server;
pub mod tools;

pub use card::server_card_json;

use std::sync::Arc;

use redis::aio::MultiplexedConnection;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp_actix_web::transport::StreamableHttpService;
use sea_orm::DatabaseConnection;

pub use server::PlaymatchMcp;

pub type PlaymatchMcpService = StreamableHttpService<PlaymatchMcp, LocalSessionManager>;

/// Build the MCP streamable HTTP service. Construct it once and clone it into
/// each actix worker so the in-memory session manager is shared across workers.
/// Mount the result with `scope("/mcp").service(service.clone().scope())`.
pub fn build_mcp_service(
	db: Arc<DatabaseConnection>,
	redis: MultiplexedConnection,
) -> PlaymatchMcpService {
	StreamableHttpService::builder()
		.service_factory(Arc::new(move || {
			Ok(PlaymatchMcp::new(db.clone(), redis.clone()))
		}))
		.session_manager(Arc::new(LocalSessionManager::default()))
		.stateful_mode(true)
		.sse_keep_alive(std::time::Duration::from_secs(20))
		.build()
}
