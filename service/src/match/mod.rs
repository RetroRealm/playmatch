use crate::db::company::get_companies_unmatched_paginator;
use crate::db::signature_metadata_mapping::{
	create_or_update_signature_metadata_mapping, SignatureMetadataMappingInput,
};
use crate::metadata::igdb::IgdbClient;
use entity::sea_orm_active_enums::{MatchTypeEnum, MetadataProviderEnum};
use log::info;
use sea_orm::DbConn;
use std::sync::Arc;
use strum::Display;

#[derive(Debug, Clone, Display)]
pub enum MetadataProvider {
	IGDB,
}

pub async fn match_db_to_igdb_entities(
	igdb_client: Arc<IgdbClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	match_companies_to_igdb_entities(igdb_client.clone(), db_conn).await?;

	Ok(())
}

pub async fn match_companies_to_igdb_entities(
	igdb_client: Arc<IgdbClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	const PAGE_SIZE: u64 = 50;
	let mut company_paginator = get_companies_unmatched_paginator(PAGE_SIZE, db_conn);

	while let Some(companies) = company_paginator.fetch_and_next().await? {
		for company_chunk in companies.chunks(4) {
			let mut results = vec![];

			for company in company_chunk.to_vec() {
				let igdb_client = igdb_client.clone();
				let db_conn = db_conn.clone();
				results.push(tokio::spawn(async move {
					match_company_to_igdb_entity(company, &db_conn, igdb_client.clone()).await
				}))
			}

			for result in results {
				result.await??;
			}
		}
	}

	Ok(())
}

pub async fn match_company_to_igdb_entity(
	company: entity::company::Model,
	db_conn: &DbConn,
	igdb_client: Arc<IgdbClient>,
) -> anyhow::Result<()> {
	let search_results = igdb_client.search_company_by_name(&company.name).await?;

	let mut matched = false;

	for search_result in search_results {
		if search_result.name.to_lowercase() == company.name.to_lowercase() {
			info!("Matched Company: {:?}", company);
			matched = true;
			create_or_update_signature_metadata_mapping(
				SignatureMetadataMappingInput {
					provider_name: MetadataProviderEnum::Igdb,
					provider_id: Some(search_result.id.to_string()),
					comment: None,
					company_id: Some(company.id),
					game_id: None,
					platform_id: None,
					match_type: MatchTypeEnum::Automatic,
					manual_match_type: None,
					failed_match_reason: None,
				},
				db_conn,
			)
			.await?;

			break;
		}
	}

	if !matched {
		info!("No direct match found for Company: {:?}", company);
		create_or_update_signature_metadata_mapping(
			SignatureMetadataMappingInput {
				provider_name: MetadataProviderEnum::Igdb,
				provider_id: None,
				comment: None,
				company_id: Some(company.id),
				game_id: None,
				platform_id: None,
				match_type: MatchTypeEnum::Failed,
				manual_match_type: None,
				failed_match_reason: Some(
					entity::sea_orm_active_enums::FailedMatchReasonEnum::NoDirectMatch,
				),
			},
			db_conn,
		)
		.await?;
	}

	Ok(())
}
