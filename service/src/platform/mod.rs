use crate::db::platform::find_all_and_join_company_and_signature_metadata_mappings;
use crate::model::PlatformResponse;
use sea_orm::DbConn;

pub async fn find_all_and_related_company_and_signature_metadata_mapping(
	db_conn: &DbConn,
) -> anyhow::Result<Vec<PlatformResponse>> {
	let platforms = find_all_and_join_company_and_signature_metadata_mappings(db_conn).await?;

	Ok(platforms
		.into_iter()
		.map(|(platform, company, mappings)| PlatformResponse {
			id: platform.id,
			name: platform.name,
			company_id: company.clone().map(|company| company.id),
			company_name: company.map(|company| company.name),
			external_metadata: mappings.into_iter().map(Into::into).collect(),
		})
		.collect())
}
