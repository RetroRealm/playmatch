use crate::db::company::{
	find_all_and_join_signature_metadata_mapping,
	find_companies_page_and_join_signature_metadata_mapping,
	get_by_id_and_join_signature_metadata_mappings,
};
use crate::db::pagination::KeysetPage;
use crate::model::CompanyMetadataResponse;
use sea_orm::DbConn;
use sea_orm::prelude::Uuid;

pub async fn get_company_by_id_and_external_metadata(
	company_id: Uuid,
	db_conn: &DbConn,
) -> anyhow::Result<Option<CompanyMetadataResponse>> {
	let result = get_by_id_and_join_signature_metadata_mappings(company_id, db_conn).await?;

	if let Some((company, mappings)) = result {
		Ok(Some(CompanyMetadataResponse {
			name: company.name,
			id: company.id,
			external_metadata: mappings.into_iter().map(Into::into).collect(),
		}))
	} else {
		Ok(None)
	}
}

pub async fn find_all_companies_and_external_metadata(
	db_conn: &DbConn,
) -> anyhow::Result<Vec<CompanyMetadataResponse>> {
	let companies = find_all_and_join_signature_metadata_mapping(db_conn).await?;

	let parsed = companies
		.into_iter()
		.map(|(company, mappings)| CompanyMetadataResponse {
			name: company.name,
			id: company.id,
			external_metadata: mappings.into_iter().map(Into::into).collect(),
		})
		.collect();

	Ok(parsed)
}

/// One keyset page of companies ordered by `(name, id)`, each mapped to the
/// public DTO with its external metadata. `after` is the last row of the
/// previous page.
pub async fn find_companies_page_and_external_metadata(
	after: Option<(String, Uuid)>,
	limit: Option<u64>,
	db_conn: &DbConn,
) -> anyhow::Result<KeysetPage<CompanyMetadataResponse>> {
	let page =
		find_companies_page_and_join_signature_metadata_mapping(after, limit, db_conn).await?;

	Ok(
		page.map_rows(|(company, mappings)| CompanyMetadataResponse {
			name: company.name,
			id: company.id,
			external_metadata: mappings.into_iter().map(Into::into).collect(),
		}),
	)
}
