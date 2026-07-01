use crate::db::stats::{
	GlobalCounts, PlatformCounts, collect_global_counts, collect_platform_counts, platform_exists,
};
use chrono::{DateTime, Utc};
use sea_orm::DbConn;
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Service-wide aggregate counts. Cheap to cache: the numbers move only when a
/// dat file is imported.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServiceStats {
	pub dat_file_count: u64,
	pub signature_group_count: u64,
	pub platform_count: u64,
	pub company_count: u64,
	pub game_count: u64,
	pub current_game_count: u64,
	pub game_file_count: u64,
	pub mapped_game_count: u64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub last_import_at: Option<DateTime<Utc>>,
}

impl From<GlobalCounts> for ServiceStats {
	fn from(value: GlobalCounts) -> Self {
		ServiceStats {
			dat_file_count: value.dat_file_count,
			signature_group_count: value.signature_group_count,
			platform_count: value.platform_count,
			company_count: value.company_count,
			game_count: value.game_count,
			current_game_count: value.current_game_count,
			game_file_count: value.game_file_count,
			mapped_game_count: value.mapped_game_count,
			last_import_at: value.last_import_at,
		}
	}
}

/// Per-platform aggregate counts.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformStats {
	pub dat_file_count: u64,
	pub current_game_count: u64,
	pub game_file_count: u64,
	pub mapped_game_count: u64,
}

impl From<PlatformCounts> for PlatformStats {
	fn from(value: PlatformCounts) -> Self {
		PlatformStats {
			dat_file_count: value.dat_file_count,
			current_game_count: value.current_game_count,
			game_file_count: value.game_file_count,
			mapped_game_count: value.mapped_game_count,
		}
	}
}

pub async fn get_service_stats(conn: &DbConn) -> anyhow::Result<ServiceStats> {
	Ok(collect_global_counts(conn).await?.into())
}

/// Per-platform stats, or `None` when the platform does not exist so the caller
/// can answer 404.
pub async fn get_platform_stats(
	platform_id: Uuid,
	conn: &DbConn,
) -> anyhow::Result<Option<PlatformStats>> {
	if !platform_exists(platform_id, conn).await? {
		return Ok(None);
	}
	Ok(Some(
		collect_platform_counts(platform_id, conn).await?.into(),
	))
}
