use crate::db::platform::get_unmatched_platforms_with_limit;
use crate::db::retroachievements::find_retroachievements_system_by_name_lower;
use crate::providers::MetadataProvider;
use crate::providers::retroachievements::RetroAchievementsClient;
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

pub async fn match_platforms_to_retroachievements(
	client: Arc<RetroAchievementsClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	let chunk_size = client.chunk_size();
	drive_match_pipeline(
		"platform",
		MetadataProviderEnum::RetroAchievements,
		get_unmatched_platforms_with_limit,
		match_platform_to_retroachievements,
		client,
		db_conn,
		chunk_size,
	)
	.await
}

pub fn match_platform_to_retroachievements(
	platform: entity::platform::Model,
	client: Arc<RetroAchievementsClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		let mut redis_conn = client.redis_conn().clone();
		if let Some(found) =
			find_retroachievements_system_by_name_lower(&platform.name, &db_conn).await?
		{
			debug!(
				"Matched Platform \"{}\" to RetroAchievements System \"{}\" id {} (Direct Match)",
				platform.name, found.name, found.system_id
			);
			write_auto_match_success(
				"retroachievements",
				MetadataProviderEnum::RetroAchievements,
				Target::Platform(platform.id),
				found.system_id.to_string(),
				AutomaticMatchReasonEnum::DirectName,
				Some(found.name),
				None,
				&db_conn,
				&mut redis_conn,
			)
			.await?;
			return Ok(());
		}

		debug!(
			"No direct match found for Platform: \"{}\" in RetroAchievements",
			&platform.name
		);
		write_auto_match_failed(
			"retroachievements",
			MetadataProviderEnum::RetroAchievements,
			Target::Platform(platform.id),
			FailedMatchReasonEnum::NoDirectMatch,
			&db_conn,
			&mut redis_conn,
		)
		.await?;

		Ok(())
	})
}
