use crate::db::platform::get_platforms_unmatched_paginator;
use crate::db::signature_metadata_mapping::{
	create_or_update_signature_metadata_mapping, SignatureMetadataMappingInput,
};
use crate::metadata::igdb::IgdbClient;
use crate::r#match::PAGE_SIZE;
use entity::sea_orm_active_enums::{AutomaticMatchReasonEnum, MatchTypeEnum, MetadataProviderEnum};
use log::debug;
use sea_orm::DbConn;
use std::sync::Arc;

pub async fn match_platforms_to_igdb(
	igdb_client: Arc<IgdbClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	let mut platform_paginator = get_platforms_unmatched_paginator(PAGE_SIZE, db_conn);

	while let Some(platforms) = platform_paginator.fetch_and_next().await? {
		for platform_chunk in platforms.chunks(4) {
			let mut results = vec![];

			for platform in platform_chunk.iter().cloned() {
				let igdb_client = igdb_client.clone();
				let db_conn = db_conn.clone();
				results.push(tokio::spawn(async move {
					match_platform_to_igdb(platform, &db_conn, igdb_client.clone()).await
				}))
			}

			for result in results {
				result.await??;
			}
		}
	}

	Ok(())
}

pub async fn match_platform_to_igdb(
	platform: entity::platform::Model,
	db_conn: &DbConn,
	igdb_client: Arc<IgdbClient>,
) -> anyhow::Result<()> {
	let search_results = igdb_client.search_platforms_by_name(&platform.name).await?;

	let mut matched = false;

	for search_result in search_results {
		if search_result.name.to_lowercase() == platform.name.to_lowercase() {
			debug!(
				"Matched Platform \"{}\" to IGDB Platform ID {} (Direct Match)",
				platform.name, search_result.id
			);
			matched = true;
			create_or_update_signature_metadata_mapping(
				SignatureMetadataMappingInput {
					provider_name: MetadataProviderEnum::Igdb,
					provider_id: Some(search_result.id.to_string()),
					comment: None,
					company_id: None,
					game_id: None,
					platform_id: Some(platform.id),
					match_type: MatchTypeEnum::Automatic,
					manual_match_type: None,
					failed_match_reason: None,
					automatic_match_reason: Some(AutomaticMatchReasonEnum::DirectName),
				},
				db_conn,
			)
			.await?;

			break;
		}
	}

	if !matched {
		debug!("No direct match found for Platform: \"{}\"", &platform.name);
		create_or_update_signature_metadata_mapping(
			SignatureMetadataMappingInput {
				provider_name: MetadataProviderEnum::Igdb,
				provider_id: None,
				comment: None,
				company_id: None,
				game_id: None,
				platform_id: Some(platform.id),
				match_type: MatchTypeEnum::Failed,
				manual_match_type: None,
				failed_match_reason: Some(
					entity::sea_orm_active_enums::FailedMatchReasonEnum::NoDirectMatch,
				),
				automatic_match_reason: None,
			},
			db_conn,
		)
		.await?;
	}

	Ok(())
}
