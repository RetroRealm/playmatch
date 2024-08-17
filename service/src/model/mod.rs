use derive_builder::Builder;
use entity::sea_orm_active_enums::{FailedMatchReasonEnum, ManualMatchModeEnum, MatchTypeEnum};
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};
use strum::EnumIter;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Serialize, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct GameFileMatchSearch {
	pub file_name: String,
	pub file_size: i64,
	pub md5: Option<String>,
	pub sha1: Option<String>,
	pub sha256: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumIter, ToSchema)]
pub enum GameMatchType {
	SHA256,
	SHA1,
	MD5,
	FileNameAndSize,
	NoMatch,
}

#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameMatchResult {
	pub game_match_type: GameMatchType,
	pub playmatch_id: Option<Uuid>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub external_metadata: Vec<ExternalMetadata>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExternalMetadata {
	pub provider_name: String,
	pub provider_id: String,
	pub match_type: MatchType,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub manual_match_type: Option<ManualMatchMode>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub failed_match_reason: Option<FailedMatchReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum MatchType {
	Automatic,
	Failed,
	Manual,
	None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum ManualMatchMode {
	Admin,
	Community,
	Trusted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum FailedMatchReason {
	TooManyMatches,
}

impl From<MatchTypeEnum> for MatchType {
	fn from(match_type: MatchTypeEnum) -> Self {
		match match_type {
			MatchTypeEnum::Automatic => MatchType::Automatic,
			MatchTypeEnum::Failed => MatchType::Failed,
			MatchTypeEnum::Manual => MatchType::Manual,
			MatchTypeEnum::None => MatchType::None,
		}
	}
}

impl From<ManualMatchModeEnum> for ManualMatchMode {
	fn from(manual_match_mode: ManualMatchModeEnum) -> Self {
		match manual_match_mode {
			ManualMatchModeEnum::Admin => ManualMatchMode::Admin,
			ManualMatchModeEnum::Community => ManualMatchMode::Community,
			ManualMatchModeEnum::Trusted => ManualMatchMode::Trusted,
		}
	}
}

impl From<FailedMatchReasonEnum> for FailedMatchReason {
	fn from(failed_match_reason: FailedMatchReasonEnum) -> Self {
		match failed_match_reason {
			FailedMatchReasonEnum::TooManyMatches => FailedMatchReason::TooManyMatches,
		}
	}
}
