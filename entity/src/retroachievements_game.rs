use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "retroachievements_game")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: Uuid,
	#[sea_orm(unique)]
	pub game_id: i64,
	pub system_id: i32,
	pub system_name: String,
	pub title: String,
	pub title_normalized: Option<String>,
	pub image_icon: Option<String>,
	pub num_achievements: i32,
	pub num_leaderboards: i32,
	pub points: i32,
	pub date_modified: Option<String>,
	pub forum_topic_id: Option<i64>,
	pub created_at: DateTimeWithTimeZone,
	pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(has_many = "super::retroachievements_game_hash::Entity")]
	Hash,
}

impl Related<super::retroachievements_game_hash::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Hash.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}
