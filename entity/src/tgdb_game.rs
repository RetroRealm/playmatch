use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "tgdb_game")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: i64,
	pub title: String,
	pub title_normalized: Option<String>,
	pub created_at: DateTimeWithTimeZone,
	pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(has_many = "super::tgdb_game_alias::Entity")]
	Alias,
}

impl Related<super::tgdb_game_alias::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Alias.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}
