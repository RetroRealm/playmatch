//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.15

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(
	rs_type = "String",
	db_type = "Enum",
	enum_name = "automatic_match_reason_enum"
)]
pub enum AutomaticMatchReasonEnum {
	#[sea_orm(string_value = "alternative_name")]
	AlternativeName,
	#[sea_orm(string_value = "direct_name")]
	DirectName,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(
	rs_type = "String",
	db_type = "Enum",
	enum_name = "failed_match_reason_enum"
)]
pub enum FailedMatchReasonEnum {
	#[sea_orm(string_value = "no_direct_match")]
	NoDirectMatch,
	#[sea_orm(string_value = "too_many_matches")]
	TooManyMatches,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(
	rs_type = "String",
	db_type = "Enum",
	enum_name = "manual_match_mode_enum"
)]
pub enum ManualMatchModeEnum {
	#[sea_orm(string_value = "admin")]
	Admin,
	#[sea_orm(string_value = "community")]
	Community,
	#[sea_orm(string_value = "trusted")]
	Trusted,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "match_type_enum")]
pub enum MatchTypeEnum {
	#[sea_orm(string_value = "automatic")]
	Automatic,
	#[sea_orm(string_value = "failed")]
	Failed,
	#[sea_orm(string_value = "manual")]
	Manual,
	#[sea_orm(string_value = "none")]
	None,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(
	rs_type = "String",
	db_type = "Enum",
	enum_name = "metadata_provider_enum"
)]
pub enum MetadataProviderEnum {
	#[sea_orm(string_value = "igdb")]
	Igdb,
}
