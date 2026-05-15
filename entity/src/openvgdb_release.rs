use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "openvgdb_release")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: Uuid,
	#[sea_orm(unique)]
	pub release_id: i64,
	pub rom_id: i64,
	pub title_name: String,
	pub region_name: Option<String>,
	pub system_name: Option<String>,
	pub cover_front: Option<String>,
	pub cover_back: Option<String>,
	pub description: Option<String>,
	pub developer: Option<String>,
	pub publisher: Option<String>,
	pub genre: Option<String>,
	pub release_date: Option<String>,
	pub release_year: Option<i32>,
	pub reference_url: Option<String>,
	pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "super::openvgdb_rom::Entity",
		from = "Column::RomId",
		to = "super::openvgdb_rom::Column::RomId"
	)]
	OpenvgdbRom,
}

impl Related<super::openvgdb_rom::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::OpenvgdbRom.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}
