use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "launchbox_game")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: Uuid,
	#[sea_orm(unique)]
	pub database_id: i64,
	pub name: String,
	pub name_normalized: Option<String>,
	pub platform_name: String,
	pub release_date: Option<String>,
	pub release_year: Option<i32>,
	pub overview: Option<String>,
	pub developer: Option<String>,
	pub publisher: Option<String>,
	pub genres: Option<String>,
	pub max_players: Option<i32>,
	pub cooperative: Option<bool>,
	pub esrb: Option<String>,
	pub release_type: Option<String>,
	pub status: Option<String>,
	pub wikipedia_url: Option<String>,
	pub video_url: Option<String>,
	pub community_rating: Option<f32>,
	pub community_rating_count: Option<i32>,
	pub created_at: DateTimeWithTimeZone,
	pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(has_many = "super::launchbox_game_alternate_name::Entity")]
	AlternateName,
	#[sea_orm(has_many = "super::launchbox_game_image::Entity")]
	Image,
}

impl Related<super::launchbox_game_alternate_name::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::AlternateName.def()
	}
}

impl Related<super::launchbox_game_image::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Image.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}
