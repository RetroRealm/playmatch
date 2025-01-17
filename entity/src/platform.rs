//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.15

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "platform")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: Uuid,
	pub name: String,
	pub company_id: Option<Uuid>,
	pub updated_at: DateTimeWithTimeZone,
	pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "super::company::Entity",
		from = "Column::CompanyId",
		to = "super::company::Column::Id",
		on_update = "NoAction",
		on_delete = "Cascade"
	)]
	Company,
	#[sea_orm(has_many = "super::dat_file::Entity")]
	DatFile,
	#[sea_orm(has_many = "super::signature_metadata_mapping::Entity")]
	SignatureMetadataMapping,
}

impl Related<super::company::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Company.def()
	}
}

impl Related<super::dat_file::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::DatFile.def()
	}
}

impl Related<super::signature_metadata_mapping::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::SignatureMetadataMapping.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}
