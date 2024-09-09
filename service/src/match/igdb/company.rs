use crate::db::company::get_unmatched_companies_with_limit;
use crate::db::signature_metadata_mapping::{
	create_or_update_signature_metadata_mapping, SignatureMetadataMappingInputBuilder,
};
use crate::metadata::igdb::IgdbClient;
use crate::r#match::igdb::IGDB_CHUNK_SIZE;
use crate::r#match::PAGE_SIZE;
use entity::sea_orm_active_enums::{
	AutomaticMatchReasonEnum, FailedMatchReasonEnum, MatchTypeEnum, MetadataProviderEnum,
};
use log::{debug, error};
use sea_orm::DbConn;
use std::sync::Arc;

pub async fn match_companies_to_igdb(
	igdb_client: Arc<IgdbClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	while let Some(inner_page) = get_unmatched_companies_with_limit(PAGE_SIZE, db_conn).await? {
		for inner_chunk in inner_page.chunks(IGDB_CHUNK_SIZE) {
			let mut results = vec![];

			for inner in inner_chunk.iter().cloned() {
				let igdb_client = igdb_client.clone();
				let db_conn = db_conn.clone();
				results.push(tokio::spawn(match_company_to_igdb(
					inner,
					igdb_client.clone(),
					db_conn,
				)));
			}

			for result in results {
				if let Err(e) = result.await? {
					error!("Error while matching company to IGDB: {:?}", e);
				}
			}
		}
	}

	Ok(())
}

async fn match_company_to_igdb(
	company: entity::company::Model,
	igdb_client: Arc<IgdbClient>,
	db_conn: DbConn,
) -> anyhow::Result<()> {
	let search_results = igdb_client.search_company_by_name(&company.name).await?;

	for search_result in search_results {
		if search_result.name.to_lowercase() == company.name.to_lowercase() {
			debug!(
				"Matched Company \"{}\" to IGDB Company ID {} (Direct Match)",
				company.name, search_result.id
			);
			create_or_update_signature_metadata_mapping(
				SignatureMetadataMappingInputBuilder::default()
					.provider(MetadataProviderEnum::Igdb)
					.provider_id(Some(search_result.id.to_string()))
					.company_id(Some(company.id))
					.match_type(MatchTypeEnum::Automatic)
					.automatic_match_reason(Some(AutomaticMatchReasonEnum::DirectName))
					.build()?,
				&db_conn,
			)
			.await?;

			return Ok(());
		}
	}

	debug!("No direct match found for Company: \"{}\"", &company.name);
	create_or_update_signature_metadata_mapping(
		SignatureMetadataMappingInputBuilder::default()
			.provider(MetadataProviderEnum::Igdb)
			.company_id(Some(company.id))
			.match_type(MatchTypeEnum::Failed)
			.failed_match_reason(Some(FailedMatchReasonEnum::NoDirectMatch))
			.build()?,
		&db_conn,
	)
	.await?;

	Ok(())
}
