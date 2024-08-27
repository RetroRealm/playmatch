use crate::db::platform::get_platforms_unmatched_paginator;
use crate::db::signature_metadata_mapping::{
	create_or_update_signature_metadata_mapping, SignatureMetadataMappingInputBuilder,
};
use crate::metadata::igdb::IgdbClient;
use crate::r#match::{handle_db_pagination_chunked, PAGE_SIZE};
use entity::sea_orm_active_enums::{
	AutomaticMatchReasonEnum, FailedMatchReasonEnum, MatchTypeEnum, MetadataProviderEnum,
};
use log::debug;
use sea_orm::DbConn;
use std::sync::Arc;

pub async fn match_platforms_to_igdb(
	igdb_client: Arc<IgdbClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	let platform_paginator = get_platforms_unmatched_paginator(PAGE_SIZE, db_conn);

	handle_db_pagination_chunked(
		platform_paginator,
		igdb_client,
		db_conn.clone(),
		|t, arc, connection| {
			tokio::spawn(async move { match_platform_to_igdb(t, arc, connection).await })
		},
	)
	.await?;

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
			create_or_update_signature_metadata_mapping(
				SignatureMetadataMappingInputBuilder::default()
					.provider(MetadataProviderEnum::Igdb)
					.provider_id(Some(search_result.id.to_string()))
					.platform_id(Some(platform.id))
					.match_type(MatchTypeEnum::Automatic)
					.automatic_match_reason(Some(AutomaticMatchReasonEnum::DirectName))
					.build()?,
				&db_conn,
			)
			.await?;

			return Ok(());
		}
	}

	debug!("No direct match found for Platform: \"{}\"", &platform.name);
	create_or_update_signature_metadata_mapping(
		SignatureMetadataMappingInputBuilder::default()
			.provider(MetadataProviderEnum::Igdb)
			.platform_id(Some(platform.id))
			.match_type(MatchTypeEnum::Failed)
			.failed_match_reason(Some(FailedMatchReasonEnum::NoDirectMatch))
			.build()?,
		&db_conn,
	)
	.await?;

	Ok(())
}
