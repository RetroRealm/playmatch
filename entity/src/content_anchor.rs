//! `SeaORM` Entity

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "content_anchor")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: Uuid,
	#[sea_orm(column_type = "Text")]
	pub anchor_hash: String,
	pub anchor_file_count: i16,
	pub created_at: DateTimeWithTimeZone,
	pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(has_many = "super::game::Entity")]
	Game,
}

impl Related<super::game::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Game.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}
