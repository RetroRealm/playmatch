use crate::db::pagination::{KeysetPage, fetch_keyset_page};
use chrono::{DateTime, Utc};
use entity::dat_file_import;
use entity::dat_file_import::Entity as DatFileImport;
use sea_orm::ActiveValue::Set;
use sea_orm::prelude::Uuid;
use sea_orm::{
	ActiveModelTrait, ColumnTrait, DbConn, DbErr, EntityTrait, PaginatorTrait, QueryFilter,
	TryIntoModel,
};

pub async fn is_dat_already_in_history(md5_hash: &str, conn: &DbConn) -> Result<bool, DbErr> {
	DatFileImport::find()
		.filter(dat_file_import::Column::Md5.eq(md5_hash))
		.count(conn)
		.await
		.map(|count| count > 0)
}

pub async fn create_dat_file_import(
	file_name: &str,
	md5_hash: &str,
	version: &str,
	dat_file_id: Uuid,
	conn: &DbConn,
) -> Result<dat_file_import::Model, DbErr> {
	let dat_file_import = dat_file_import::ActiveModel {
		dat_file_id: Set(dat_file_id),
		name: Set(file_name.to_string()),
		version: Set(version.to_string()),
		md5: Set(md5_hash.to_string()),
		..Default::default()
	};

	dat_file_import.save(conn).await?.try_into_model()
}

pub async fn get_dat_file_import_by_id(
	id: Uuid,
	conn: &DbConn,
) -> Result<Option<dat_file_import::Model>, DbErr> {
	DatFileImport::find_by_id(id).one(conn).await
}

/// Fetch one keyset page of a dat file's imports ordered by `(imported_at, id)`
/// descending so the newest import leads. Seeks past `after` when supplied. The
/// N+1 overflow row drives `has_more`.
pub async fn find_imports_for_dat_file_page(
	dat_file_id: Uuid,
	after: Option<(DateTime<Utc>, Uuid)>,
	limit: Option<u64>,
	conn: &DbConn,
) -> Result<KeysetPage<dat_file_import::Model>, DbErr> {
	let mut cursor = DatFileImport::find()
		.filter(dat_file_import::Column::DatFileId.eq(dat_file_id))
		.cursor_by((
			dat_file_import::Column::ImportedAt,
			dat_file_import::Column::Id,
		));
	cursor.desc();
	if let Some((imported_at, id)) = after {
		cursor.after((imported_at, id));
	}
	fetch_keyset_page(&mut cursor, limit, conn).await
}
