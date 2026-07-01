use crate::db::pagination::{KeysetPage, fetch_keyset_page};
use entity::signature_group;
use entity::signature_group::Model as SignatureGroup;
use sea_orm::prelude::Uuid;
use sea_orm::sea_query::Expr;
use sea_orm::sea_query::extension::postgres::PgExpr;
use sea_orm::{ColumnTrait, DbConn, DbErr, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use std::collections::HashMap;

/// Looks up a signature group by exact, case-sensitive name.
pub async fn find_signature_group_by_name(
	name: &str,
	conn: &DbConn,
) -> Result<Option<SignatureGroup>, DbErr> {
	signature_group::Entity::find()
		.filter(signature_group::Column::Name.eq(name))
		.one(conn)
		.await
}

pub async fn find_all_signature_groups(conn: &DbConn) -> Result<Vec<SignatureGroup>, DbErr> {
	signature_group::Entity::find()
		.order_by_asc(signature_group::Column::Name)
		.all(conn)
		.await
}

pub async fn find_signature_group_by_id(
	id: Uuid,
	conn: &DbConn,
) -> Result<Option<SignatureGroup>, DbErr> {
	signature_group::Entity::find_by_id(id).one(conn).await
}

/// Bulk variant of [`find_signature_group_by_id`]. Loads every signature group
/// in `ids` in one `is_in` query, keyed by id.
pub async fn find_signature_groups_by_ids(
	ids: &[Uuid],
	conn: &DbConn,
) -> Result<HashMap<Uuid, SignatureGroup>, DbErr> {
	if ids.is_empty() {
		return Ok(HashMap::new());
	}
	let rows = signature_group::Entity::find()
		.filter(signature_group::Column::Id.is_in(ids.iter().copied()))
		.all(conn)
		.await?;
	Ok(rows.into_iter().map(|row| (row.id, row)).collect())
}

pub async fn find_signature_groups_page(
	after: Option<(String, Uuid)>,
	limit: Option<u64>,
	conn: &DbConn,
) -> Result<KeysetPage<SignatureGroup>, DbErr> {
	let mut cursor = signature_group::Entity::find()
		.cursor_by((signature_group::Column::Name, signature_group::Column::Id));
	if let Some((name, id)) = after {
		cursor.after((name, id));
	}
	fetch_keyset_page(&mut cursor, limit, conn).await
}

/// Fetch one keyset page of signature groups whose name contains `query`,
/// matched case-insensitively. Ordered and joined exactly like
/// [`find_signature_groups_page`] so the same `(name, id)` cursor seeks
/// deterministically across both endpoints. Seeks past `after` when supplied.
pub async fn search_signature_groups_by_name_page(
	query: &str,
	after: Option<(String, Uuid)>,
	limit: Option<u64>,
	conn: &DbConn,
) -> Result<KeysetPage<SignatureGroup>, DbErr> {
	let pattern = format!("%{query}%");
	let mut cursor = signature_group::Entity::find()
		.filter(Expr::col(signature_group::Column::Name).ilike(pattern))
		.cursor_by((signature_group::Column::Name, signature_group::Column::Id));
	if let Some((name, id)) = after {
		cursor.after((name, id));
	}
	fetch_keyset_page(&mut cursor, limit, conn).await
}

/// Count every signature group. Cheap on this small bounded reference table; the
/// v2 list opts into this only when the caller asks for a total.
pub async fn count_signature_groups(conn: &DbConn) -> Result<u64, DbErr> {
	signature_group::Entity::find().count(conn).await
}
