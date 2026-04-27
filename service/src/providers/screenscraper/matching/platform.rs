use crate::db::platform::get_unmatched_platforms_with_limit;
use crate::providers::MetadataProvider;
use crate::providers::screenscraper::ScreenScraperClient;
use crate::providers::{
	Target, drive_match_pipeline, write_auto_match_failed, write_auto_match_success,
};
use entity::sea_orm_active_enums::{
	AutomaticMatchReasonEnum, FailedMatchReasonEnum, MetadataProviderEnum,
};
use futures_util::future::BoxFuture;
use log::debug;
use sea_orm::DbConn;
use std::sync::Arc;

pub async fn match_platforms_to_screenscraper(
	client: Arc<ScreenScraperClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	let chunk_size = client.chunk_size();
	drive_match_pipeline(
		"platform",
		MetadataProviderEnum::Screenscraper,
		get_unmatched_platforms_with_limit,
		match_platform_to_screenscraper,
		client,
		db_conn,
		chunk_size,
	)
	.await
}

pub fn match_platform_to_screenscraper(
	platform: entity::platform::Model,
	client: Arc<ScreenScraperClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		if client.is_quota_exhausted() {
			return Ok(());
		}

		let mut redis_conn = client.redis_conn().clone();
		let systems = client.list_systems().await?;
		let target_lower = platform.name.to_lowercase();

		for system in systems.iter() {
			for name in system.iter_names() {
				if name.to_lowercase() == target_lower {
					debug!(
						"Matched Platform \"{}\" to ScreenScraper System ID {} (Direct Match)",
						platform.name, system.id
					);
					write_auto_match_success(
						"screenscraper",
						MetadataProviderEnum::Screenscraper,
						Target::Platform(platform.id),
						system.id.to_string(),
						AutomaticMatchReasonEnum::DirectName,
						&db_conn,
						&mut redis_conn,
					)
					.await?;
					return Ok(());
				}
			}
		}

		debug!(
			"No direct match found for Platform: \"{}\" in ScreenScraper",
			&platform.name
		);
		write_auto_match_failed(
			"screenscraper",
			MetadataProviderEnum::Screenscraper,
			Target::Platform(platform.id),
			FailedMatchReasonEnum::NoDirectMatch,
			&db_conn,
			&mut redis_conn,
		)
		.await?;

		Ok(())
	})
}
