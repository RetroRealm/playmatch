use crate::sea_orm::{
	ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, Set,
};
use entity::signature_group;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		// Get the connection and start a transaction
		let db = manager.get_connection();
		let no_intro = signature_group::ActiveModel {
			name: Set("DatsSite-Legacy".to_string()),
			website_link: Set(Some("https://dats.site/".to_string())),
			description: Set(Some("Legacy dat files from dats.site, these legacy old DATs included special compressed versions of Redump files, RVZ (GameCube/Wii) and WUX (Wii U). While no longer hosted (any updated) on dats.site, they're still handy to keep around.".to_string())),
			..Default::default()
		};

		no_intro.save(db).await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let db = manager.get_connection();

		let group = signature_group::Entity::find()
			.filter(signature_group::Column::Name.eq("DatsSite-Legacy"))
			.one(db)
			.await?;

		if let Some(group) = group {
			signature_group::Entity::delete(group.into_active_model())
				.exec(db)
				.await?;
		}

		Ok(())
	}
}
