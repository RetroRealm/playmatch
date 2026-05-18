use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "tgdb_game_alias")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: Uuid,
	pub game_id: i64,
	pub alt_name: String,
	pub alt_name_normalized: Option<String>,
	pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "super::tgdb_game::Entity",
		from = "Column::GameId",
		to = "super::tgdb_game::Column::Id"
	)]
	TgdbGame,
}

impl Related<super::tgdb_game::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::TgdbGame.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}
