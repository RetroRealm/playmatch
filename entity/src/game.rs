//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.15

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "game")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: Uuid,
	pub dat_file_import_id: Uuid,
	pub signature_group_internal_id: Option<String>,
	pub name: String,
	pub description: Option<String>,
	pub categories: Option<Vec<String>>,
	pub clone_of: Option<Uuid>,
	pub created_at: DateTimeWithTimeZone,
	pub updated_at: DateTimeWithTimeZone,
	#[sea_orm(column_type = "Text", nullable)]
	pub signature_group_internal_clone_of_id: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "super::dat_file_import::Entity",
		from = "Column::DatFileImportId",
		to = "super::dat_file_import::Column::Id",
		on_update = "NoAction",
		on_delete = "Cascade"
	)]
	DatFileImport,
	#[sea_orm(
		belongs_to = "Entity",
		from = "Column::CloneOf",
		to = "Column::Id",
		on_update = "NoAction",
		on_delete = "Cascade"
	)]
	SelfRef,
	#[sea_orm(has_many = "super::game_file::Entity")]
	GameFile,
	#[sea_orm(has_many = "super::signature_metadata_mapping::Entity")]
	SignatureMetadataMapping,
}

impl Related<super::dat_file_import::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::DatFileImport.def()
	}
}

impl Related<super::game_file::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::GameFile.def()
	}
}

impl Related<super::signature_metadata_mapping::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::SignatureMetadataMapping.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}
