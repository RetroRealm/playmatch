use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "retroachievements_game_hash")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: Uuid,
	pub game_id: i64,
	pub md5: String,
	pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "super::retroachievements_game::Entity",
		from = "Column::GameId",
		to = "super::retroachievements_game::Column::GameId"
	)]
	RetroachievementsGame,
}

impl Related<super::retroachievements_game::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::RetroachievementsGame.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}
