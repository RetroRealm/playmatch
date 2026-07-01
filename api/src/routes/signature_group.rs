use crate::error;
use crate::routes::ok_or_not_found;
use actix_web::web::{Data, Path};
use actix_web::{HttpResponse, Responder, get};
use sea_orm::DatabaseConnection;
use service::entities::signature_group::{find_all_signature_groups, find_signature_group_by_id};
use uuid::Uuid;

/// Lists all signature groups ordered by name.
#[utoipa::path(
	get,
	tag = "Signature Group",
	responses(
		(status = 200, description = "The full list of signature groups ordered by name", body = Vec<PlaymatchSignatureGroup>)
	)
)]
#[get("/signature-groups")]
pub async fn get_all_signature_groups(
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let signature_groups = find_all_signature_groups(db_conn.get_ref()).await?;

	Ok(HttpResponse::Ok().json(signature_groups))
}

/// Returns a signature group by id.
#[utoipa::path(
	get,
	tag = "Signature Group",
	responses(
		(status = 200, description = "The signature group", body = PlaymatchSignatureGroup),
		(status = 404, description = "Signature group not found")
	)
)]
#[get("/signature-groups/{id}")]
pub async fn get_signature_group_by_id(
	id: Path<Uuid>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let signature_group = find_signature_group_by_id(id.into_inner(), db_conn.get_ref()).await?;

	Ok(ok_or_not_found(signature_group))
}
