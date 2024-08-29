use crate::db::abstraction::ColumnNullTrait;
use entity::dat_file;
use entity::prelude::DatFile;
use sea_orm::prelude::Uuid;
use sea_orm::ActiveValue::Set;
use sea_orm::{
	ActiveModelTrait, ColumnTrait, DbConn, EntityTrait, IntoActiveModel, QueryFilter, TryIntoModel,
};

pub struct DatFileCreateOrUpdateInput {
	pub signature_group_id: Uuid,
	pub sanitized_file_name: String,
	pub current_version: String,
	pub tags: Vec<String>,
	pub subset: Option<String>,
	pub company_id: Option<Uuid>,
	pub platform_id: Uuid,
}

pub async fn find_all_dat_files(conn: &DbConn) -> anyhow::Result<Vec<dat_file::Model>> {
	Ok(DatFile::find().all(conn).await?)
}

pub async fn create_or_update_dat_file(
	input: DatFileCreateOrUpdateInput,
	conn: &DbConn,
) -> anyhow::Result<dat_file::Model> {
	let dat_file = DatFile::find()
		.filter(dat_file::Column::SignatureGroupId.eq(input.signature_group_id))
		.filter(dat_file::Column::Name.eq(input.sanitized_file_name.clone()))
		.filter(dat_file::Column::CompanyId.eq_null(input.company_id))
		.filter(dat_file::Column::PlatformId.eq(input.platform_id))
		.one(conn)
		.await?;

	if let Some(dat_file) = dat_file {
		if dat_file.current_version != input.current_version {
			let mut active_model = dat_file.into_active_model();
			active_model.current_version = Set(input.current_version.to_string());

			return Ok(active_model.save(conn).await?.try_into_model()?);
		}

		return Ok(dat_file);
	}

	let dat_file = dat_file::ActiveModel {
		signature_group_id: Set(input.signature_group_id),
		name: Set(input.sanitized_file_name.clone()),
		current_version: Set(input.current_version.clone()),
		company_id: Set(input.company_id),
		platform_id: Set(input.platform_id),
		tags: Set(if input.tags.is_empty() {
			None
		} else {
			Some(input.tags)
		}),
		subset: Set(input.subset),
		..Default::default()
	};

	Ok(dat_file.save(conn).await?.try_into_model()?)
}
