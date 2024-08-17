use entity::dat_file;
use entity::prelude::DatFile;
use sea_orm::prelude::Uuid;
use sea_orm::ActiveValue::Set;
use sea_orm::{
	ActiveModelTrait, ColumnTrait, DbConn, EntityTrait, IntoActiveModel, QueryFilter, TryIntoModel,
};

pub async fn create_or_update_dat_file(
	signature_group_id: &Uuid,
	file_name: &str,
	current_version: &str,
	tags: Vec<String>,
	subset: Option<String>,
	company_id: Option<Uuid>,
	platform_id: &Uuid,
	conn: &DbConn,
) -> anyhow::Result<dat_file::Model> {
	let dat_file = DatFile::find()
		.filter(dat_file::Column::SignatureGroupId.eq(signature_group_id.clone()))
		.filter(dat_file::Column::Name.eq(file_name))
		.filter(dat_file::Column::CompanyId.eq(company_id))
		.filter(dat_file::Column::PlatformId.eq(platform_id.clone()))
		.one(conn)
		.await?;

	if let Some(dat_file) = dat_file {
		if dat_file.current_version != current_version {
			let mut active_model = dat_file.into_active_model();
			active_model.current_version = Set(current_version.to_string());

			return Ok(active_model.save(conn).await?.try_into_model()?);
		}

		return Ok(dat_file);
	}

	let dat_file = dat_file::ActiveModel {
		signature_group_id: Set(signature_group_id.clone()),
		name: Set(file_name.to_string()),
		current_version: Set(current_version.to_string()),
		company_id: Set(company_id.clone()),
		platform_id: Set(platform_id.clone()),
		tags: Set(if tags.is_empty() { None } else { Some(tags) }),
		subset: Set(subset),
		..Default::default()
	};

	Ok(dat_file.save(conn).await?.try_into_model()?)
}
