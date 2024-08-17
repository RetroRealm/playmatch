//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.15

use super::sea_orm_active_enums::FailedMatchReasonEnum;
use super::sea_orm_active_enums::ManualMatchModeEnum;
use super::sea_orm_active_enums::MatchTypeEnum;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "signature_metadata_mapping")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: Uuid,
	pub game_id: Option<Uuid>,
	pub company_id: Option<Uuid>,
	pub platform_id: Option<Uuid>,
	pub provider_name: String,
	pub provider_id: String,
	pub match_type: MatchTypeEnum,
	pub manual_match_type: Option<ManualMatchModeEnum>,
	pub failed_match_reason: Option<FailedMatchReasonEnum>,
	pub comment: Option<String>,
	pub created_at: DateTimeWithTimeZone,
	pub updated_at: DateTimeWithTimeZone,
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
	#[sea_orm(
		belongs_to = "super::game::Entity",
		from = "Column::GameId",
		to = "super::game::Column::Id",
		on_update = "NoAction",
		on_delete = "Cascade"
	)]
	Game,
	#[sea_orm(
		belongs_to = "super::platform::Entity",
		from = "Column::PlatformId",
		to = "super::platform::Column::Id",
		on_update = "NoAction",
		on_delete = "Cascade"
	)]
	Platform,
}

impl Related<super::company::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Company.def()
	}
}

impl Related<super::game::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Game.def()
	}
}

impl Related<super::platform::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Platform.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}
