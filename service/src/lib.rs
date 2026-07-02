//! Core domain crate of Playmatch: DAT file ingestion, the identify cascade,
//! the provider matching pipeline, the database query layer, and Redis
//! caching. The api crate exposes this over HTTP and the mcp crate exposes it
//! to MCP clients.

pub mod bulk;
pub mod cache;
pub mod config;
pub mod db;
pub mod entities;
pub mod error;
pub mod external_suggestion;
mod fs;
pub mod http;
pub mod identification;
pub mod ingestion;
pub mod matching;
pub mod metrics;
pub mod model;
pub mod providers;
