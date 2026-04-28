use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "launchbox_platform")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: Uuid,
	pub name: String,
	pub emulated: Option<bool>,
	pub release_date: Option<String>,
	pub developer: Option<String>,
	pub manufacturer: Option<String>,
	pub cpu: Option<String>,
	pub memory: Option<String>,
	pub graphics: Option<String>,
	pub sound: Option<String>,
	pub display: Option<String>,
	pub media: Option<String>,
	pub max_controllers: Option<String>,
	pub notes: Option<String>,
	pub category: Option<String>,
	pub created_at: DateTimeWithTimeZone,
	pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
