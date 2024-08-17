use entity::dat_file_import;
use entity::dat_file_import::Entity as DatFileImport;
use sea_orm::prelude::Uuid;
use sea_orm::ActiveValue::Set;
use sea_orm::{
	ActiveModelTrait, ColumnTrait, DbConn, DbErr, EntityTrait, PaginatorTrait, QueryFilter,
	TryIntoModel,
};

pub async fn is_dat_already_in_history(md5_hash: &str, conn: &DbConn) -> Result<bool, DbErr> {
	DatFileImport::find()
		.filter(dat_file_import::Column::Md5Hash.eq(md5_hash))
		.count(conn)
		.await
		.map(|count| count > 0)
}

pub async fn create_dat_file_import(
	md5_hash: &str,
	version: &str,
	dat_file_id: &Uuid,
	conn: &DbConn,
) -> Result<dat_file_import::Model, DbErr> {
	let dat_file_import = dat_file_import::ActiveModel {
		dat_file_id: Set(dat_file_id.clone()),
		version: Set(version.to_string()),
		md5_hash: Set(md5_hash.to_string()),
		..Default::default()
	};

	Ok(dat_file_import.save(conn).await?.try_into_model()?)
}
