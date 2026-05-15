use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "openvgdb_rom")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: Uuid,
	#[sea_orm(unique)]
	pub rom_id: i64,
	pub system_id: Option<i32>,
	pub region_id: Option<i32>,
	pub rom_hash_crc: Option<String>,
	pub rom_hash_md5: Option<String>,
	pub rom_hash_sha1: Option<String>,
	pub rom_size: Option<i64>,
	pub rom_file_name: Option<String>,
	pub rom_extensionless_file_name: Option<String>,
	pub rom_serial: Option<String>,
	pub created_at: DateTimeWithTimeZone,
	pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(has_many = "super::openvgdb_release::Entity")]
	Release,
}

impl Related<super::openvgdb_release::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Release.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}
