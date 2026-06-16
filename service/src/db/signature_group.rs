use entity::signature_group;
use entity::signature_group::Model as SignatureGroup;
use sea_orm::prelude::Uuid;
use sea_orm::{ColumnTrait, DbConn, DbErr, EntityTrait, QueryFilter, QueryOrder};

/// Find a signature group by exact (case-sensitive) name.
pub async fn find_signature_group_by_name(
	name: &str,
	conn: &DbConn,
) -> Result<Option<SignatureGroup>, DbErr> {
	signature_group::Entity::find()
		.filter(signature_group::Column::Name.eq(name))
		.one(conn)
		.await
}

/// Every signature group, ordered by name.
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
