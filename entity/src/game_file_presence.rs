//! `SeaORM` Entity

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "game_file_presence")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: Uuid,
	pub game_file_id: Uuid,
	pub dat_file_import_id: Uuid,
	pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "super::game_file::Entity",
		from = "Column::GameFileId",
		to = "super::game_file::Column::Id",
		on_update = "NoAction",
		on_delete = "Cascade"
	)]
	GameFile,
	#[sea_orm(
		belongs_to = "super::dat_file_import::Entity",
		from = "Column::DatFileImportId",
		to = "super::dat_file_import::Column::Id",
		on_update = "NoAction",
		on_delete = "Cascade"
	)]
	DatFileImport,
}

impl Related<super::game_file::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::GameFile.def()
	}
}

impl Related<super::dat_file_import::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::DatFileImport.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}
