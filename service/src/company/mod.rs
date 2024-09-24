use crate::db::company::find_all_and_join_signature_metadata_mapping;
use crate::model::CompanyResponse;
use sea_orm::DbConn;

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
