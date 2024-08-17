//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.15

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "dat_file_import")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: Uuid,
	pub dat_file_id: Uuid,
	pub version: String,
	pub md5_hash: String,
	pub imported_at: DateTimeWithTimeZone,
	pub created_at: DateTimeWithTimeZone,
	pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "super::dat_file::Entity",
		from = "Column::DatFileId",
		to = "super::dat_file::Column::Id",
		on_update = "NoAction",
		on_delete = "Cascade"
	)]
	DatFile,
	#[sea_orm(has_many = "super::game::Entity")]
	Game,
}

impl Related<super::dat_file::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::DatFile.def()
	}
}

impl Related<super::game::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Game.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}