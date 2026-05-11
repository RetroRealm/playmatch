use crate::db::platform::get_unmatched_platforms_with_limit;
use crate::providers::MetadataProvider;
use crate::providers::igdb::IgdbClient;
use crate::providers::{
	DEFAULT_CHUNK_SIZE, Target, drive_match_pipeline, write_auto_match_failed,
	write_auto_match_success,
};
use entity::sea_orm_active_enums::{
	AutomaticMatchReasonEnum, FailedMatchReasonEnum, MetadataProviderEnum,
};
use futures_util::future::BoxFuture;
use log::debug;
use sea_orm::DbConn;
use std::sync::Arc;

pub async fn match_platforms_to_igdb(
	igdb_client: Arc<IgdbClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	drive_match_pipeline(
		"platform",
		MetadataProviderEnum::Igdb,
		get_unmatched_platforms_with_limit,
		match_platform_to_igdb,
		igdb_client,
		db_conn,
		DEFAULT_CHUNK_SIZE,
	)
	.await
}

pub fn match_platform_to_igdb(
	platform: entity::platform::Model,
	igdb_client: Arc<IgdbClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		let mut redis_conn = igdb_client.redis_conn().clone();
		let search_results = igdb_client.search_platforms_by_name(&platform.name).await?;

		for search_result in search_results {
			if search_result.name.to_lowercase() == platform.name.to_lowercase() {
				debug!(
					"Matched Platform \"{}\" to IGDB Platform ID {} (Direct Match)",
					platform.name, search_result.id
				);
				write_auto_match_success(
					"igdb",
					MetadataProviderEnum::Igdb,
					Target::Platform(platform.id),
					search_result.id.to_string(),
					AutomaticMatchReasonEnum::DirectName,
					Some(search_result.name.clone()),
					None,
					&db_conn,
					&mut redis_conn,
				)
				.await?;

				return Ok(());
			}
		}

		debug!("No direct match found for Platform: \"{}\"", &platform.name);
		write_auto_match_failed(
			"igdb",
			MetadataProviderEnum::Igdb,
			Target::Platform(platform.id),
			FailedMatchReasonEnum::NoDirectMatch,
			&db_conn,
			&mut redis_conn,
		)
		.await?;

		Ok(())
	})
}
