use entity::signature_group;
use entity::signature_group::Model as SignatureGroup;
use sea_orm::{ColumnTrait, DbConn, DbErr, EntityTrait, QueryFilter};

pub async fn find_signature_group_by_name(
	name: &str,
	conn: &DbConn,
) -> Result<Option<SignatureGroup>, DbErr> {
	signature_group::Entity::find()
		.filter(signature_group::Column::Name.eq(name))
		.one(conn)
		.await
}
