use crate::db::company::{
	find_all_and_join_signature_metadata_mapping, get_by_id_and_join_signature_metadata_mappings,
};
use crate::model::CompanyResponse;
use sea_orm::prelude::Uuid;
use sea_orm::DbConn;

pub async fn get_company_by_id_and_external_metadata(
	company_id: Uuid,
	db_conn: &DbConn,
) -> anyhow::Result<Option<CompanyResponse>> {
	let result = get_by_id_and_join_signature_metadata_mappings(company_id, db_conn).await?;

	if let Some((company, mappings)) = result {
		Ok(Some(CompanyResponse {
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
) -> anyhow::Result<Vec<CompanyResponse>> {
	let companies = find_all_and_join_signature_metadata_mapping(db_conn).await?;

	let parsed = companies
		.into_iter()
		.map(|(company, mappings)| CompanyResponse {
			name: company.name,
			id: company.id,
			external_metadata: mappings.into_iter().map(Into::into).collect(),
		})
		.collect();

	Ok(parsed)
}
