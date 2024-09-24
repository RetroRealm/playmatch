use crate::db::platform::{
	find_all_and_join_company_and_signature_metadata_mappings,
	get_by_id_and_join_company_and_signature_metadata_mappings,
};
use crate::model::PlatformResponse;
use sea_orm::prelude::Uuid;
use sea_orm::DbConn;

pub async fn get_platform_by_id_and_related_company_and_signature_metadata_mapping(
	id: Uuid,
	db_conn: &DbConn,
) -> anyhow::Result<Option<PlatformResponse>> {
	let platform = get_by_id_and_join_company_and_signature_metadata_mappings(id, db_conn).await?;

	if let Some((platform, company, mappings)) = platform {
		Ok(Some(PlatformResponse {
			id: platform.id,
			name: platform.name,
			company_id: company.clone().map(|company| company.id),
			company_name: company.map(|company| company.name),
			external_metadata: mappings.into_iter().map(Into::into).collect(),
		}))
	} else {
		Ok(None)
	}
}

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
