use crate::db::platform::get_unmatched_platforms_with_limit;
use crate::providers::igdb::IgdbClient;
use crate::providers::igdb::matching::{
	IGDB_CHUNK_SIZE, PAGE_SIZE, Target, write_auto_match_failed, write_auto_match_success,
};
use entity::sea_orm_active_enums::{
	AutomaticMatchReasonEnum, FailedMatchReasonEnum, MetadataProviderEnum,
};
use log::{debug, error};
use sea_orm::DbConn;
use std::sync::Arc;

pub async fn match_platforms_to_igdb(
	igdb_client: Arc<IgdbClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	while let Some(inner_page) =
		get_unmatched_platforms_with_limit(MetadataProviderEnum::Igdb, PAGE_SIZE, db_conn).await?
	{
		for inner_chunk in inner_page.chunks(IGDB_CHUNK_SIZE) {
			let mut results = vec![];

			for inner in inner_chunk.iter().cloned() {
				let igdb_client = igdb_client.clone();
				let db_conn = db_conn.clone();
				results.push(tokio::spawn(match_platform_to_igdb(
					inner,
					igdb_client.clone(),
					db_conn,
				)));
			}

			for result in results {
				if let Err(e) = result.await? {
					error!("Error while matching platform to IGDB: {e:?}");
				}
			}
		}
	}

	Ok(())
}

pub async fn match_platform_to_igdb(
	platform: entity::platform::Model,
	igdb_client: Arc<IgdbClient>,
	db_conn: DbConn,
) -> anyhow::Result<()> {
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
				&db_conn,
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
	)
	.await?;

	Ok(())
}
