use crate::model::PlaymatchSignatureGroup;
use sea_orm::DbConn;
use sea_orm::prelude::Uuid;

pub async fn find_all_signature_groups(
	db_conn: &DbConn,
) -> anyhow::Result<Vec<PlaymatchSignatureGroup>> {
	let groups = crate::db::signature_group::find_all_signature_groups(db_conn).await?;
	Ok(groups.into_iter().map(Into::into).collect())
}

pub async fn find_signature_group_by_id(
	id: Uuid,
	db_conn: &DbConn,
) -> anyhow::Result<Option<PlaymatchSignatureGroup>> {
	let group = crate::db::signature_group::find_signature_group_by_id(id, db_conn).await?;
	Ok(group.map(Into::into))
}
