use crate::db::launchbox::find_lb_platform_by_name_lower;
use crate::db::platform::get_unmatched_platforms_with_limit;
use crate::providers::MetadataProvider;
use crate::providers::launchbox::LaunchBoxClient;
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

pub async fn match_platforms_to_launchbox(
	client: Arc<LaunchBoxClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	let chunk_size = client.chunk_size();
	drive_match_pipeline(
		"platform",
		MetadataProviderEnum::Launchbox,
		get_unmatched_platforms_with_limit,
		match_platform_to_launchbox,
		client,
		db_conn,
		chunk_size,
	)
	.await
}

pub fn match_platform_to_launchbox(
	platform: entity::platform::Model,
	client: Arc<LaunchBoxClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		let mut redis_conn = client.redis_conn().clone();
		if let Some(found) = find_lb_platform_by_name_lower(&platform.name, &db_conn).await? {
			debug!(
				"Matched Platform \"{}\" to LaunchBox Platform \"{}\" (Direct Match)",
				platform.name, found.name
			);
			write_auto_match_success(
				"launchbox",
				MetadataProviderEnum::Launchbox,
				Target::Platform(platform.id),
				found.name,
				AutomaticMatchReasonEnum::DirectName,
				&db_conn,
				&mut redis_conn,
			)
			.await?;
			return Ok(());
		}

		debug!(
			"No direct match found for Platform: \"{}\" in LaunchBox",
			&platform.name
		);
		write_auto_match_failed(
			"launchbox",
			MetadataProviderEnum::Launchbox,
			Target::Platform(platform.id),
			FailedMatchReasonEnum::NoDirectMatch,
			&db_conn,
			&mut redis_conn,
		)
		.await?;

		Ok(())
	})
}
