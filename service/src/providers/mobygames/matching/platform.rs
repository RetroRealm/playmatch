use crate::db::platform::get_unmatched_platforms_with_limit;
use crate::providers::MetadataProvider;
use crate::providers::mobygames::MobyGamesClient;
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

pub async fn match_platforms_to_mobygames(
	client: Arc<MobyGamesClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	let chunk_size = client.chunk_size();
	drive_match_pipeline(
		"platform",
		MetadataProviderEnum::Mobygames,
		get_unmatched_platforms_with_limit,
		match_platform_to_mobygames,
		client,
		db_conn,
		chunk_size,
	)
	.await
}

pub fn match_platform_to_mobygames(
	platform: entity::platform::Model,
	client: Arc<MobyGamesClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		let mut redis_conn = client.redis_conn().clone();
		let platforms = client.list_platforms().await?;
		let target_lower = platform.name.to_lowercase();

		for candidate in platforms.iter() {
			if candidate.platform_name.to_lowercase() == target_lower {
				debug!(
					"Matched Platform \"{}\" to MobyGames Platform ID {} (Direct Match)",
					platform.name, candidate.platform_id
				);
				write_auto_match_success(
					"mobygames",
					MetadataProviderEnum::Mobygames,
					Target::Platform(platform.id),
					candidate.platform_id.to_string(),
					AutomaticMatchReasonEnum::DirectName,
					Some(candidate.platform_name.clone()),
					&db_conn,
					&mut redis_conn,
				)
				.await?;
				return Ok(());
			}
		}

		debug!(
			"No direct match found for Platform: \"{}\" in MobyGames",
			&platform.name
		);
		write_auto_match_failed(
			"mobygames",
			MetadataProviderEnum::Mobygames,
			Target::Platform(platform.id),
			FailedMatchReasonEnum::NoDirectMatch,
			&db_conn,
			&mut redis_conn,
		)
		.await?;

		Ok(())
	})
}
