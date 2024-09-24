use derive_builder::Builder;
use entity::sea_orm_active_enums::{
	AutomaticMatchReasonEnum, FailedMatchReasonEnum, ManualMatchModeEnum, MatchTypeEnum,
	MetadataProviderEnum,
};
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};
use strum::EnumIter;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Serialize, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct GameFileMatchSearch {
	/// The file name of the game file.
	pub file_name: String,

	/// The size of the game file in bytes.
	pub file_size: i64,

	/// Optional MD5 hash of the game file.
	pub md5: Option<String>,

	/// Optional SHA1 hash of the game file.
	pub sha1: Option<String>,

	/// Optional SHA256 hash of the game file.
	pub sha256: Option<String>,
}

/// Type of match for this game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumIter, ToSchema)]
pub enum GameMatchType {
	/// Matched by SHA256 hash.
	SHA256,

	/// Matched by SHA1 hash.
	SHA1,

	/// Matched by MD5 hash.
	MD5,

	/// Matched by file name and size.
	FileNameAndSize,

	/// No match found.
	NoMatch,
}

/// Result of a game match.
#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameMatchResult {
	/// The type of match that was found.
	pub game_match_type: GameMatchType,

	/// If a match was found, the ID of the matched game.
	pub id: Option<Uuid>,

	/// External metadata for the matched game.
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub external_metadata: Vec<ExternalMetadata>,
}

/// Response for a company including external metadata.
#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompanyResponse {
	/// The ID of the company.
	pub id: Uuid,

	/// The name of the company.
	pub name: String,

	/// External metadata for the company.
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub external_metadata: Vec<ExternalMetadata>,
}

/// Response for a platform including external metadata.
#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformResponse {
	/// The ID of the platform.
	pub id: Uuid,

	/// The name of the platform.
	pub name: String,

	/// Optional name of the company that made the platform.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub company_name: Option<String>,

	/// Optional ID of the company that made the platform.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub company_id: Option<Uuid>,

	/// External metadata for the platform.
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub external_metadata: Vec<ExternalMetadata>,
}

/// External metadata for a game/platform/company.
#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExternalMetadata {
	/// The Name of the metadata provider.
	pub provider_name: MetadataProvider,

	/// The ID of the game for this provider.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub provider_id: Option<String>,

	/// Type of how this game was matched to this Provider
	pub match_type: MatchType,

	/// Optional Comment about the match.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub comment: Option<String>,

	/// Optional Type of manual match
	#[serde(skip_serializing_if = "Option::is_none")]
	pub manual_match_type: Option<ManualMatchMode>,

	/// Optional Reason why the match failed
	#[serde(skip_serializing_if = "Option::is_none")]
	pub failed_match_reason: Option<FailedMatchReason>,

	/// Optional Reason for automatic match
	#[serde(skip_serializing_if = "Option::is_none")]
	pub automatic_match_reason: Option<AutomaticMatchReason>,
}

/// Metadata provider for game/platform/company.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum MetadataProvider {
	/// IGDB (https://www.igdb.com/)
	IGDB,
}

/// Match types for a game
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum MatchType {
	/// The game was automatically matched.
	Automatic,

	/// Automatic game matching failed and no manual match was done.
	Failed,

	/// The game was manually matched
	Manual,

	/// No match was done.
	None,
}

/// How a game was manually matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum ManualMatchMode {
	/// Game was manually matched by an Admin, which is the most trusted match.
	Admin,

	/// Game was manually matched by the community, via Discord.
	Community,

	/// Game was manually matched by a trusted user
	Trusted,
}

/// Reason why an automatic match failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum FailedMatchReason {
	/// No direct match was found.
	NoDirectMatch,

	/// Too many direct matches were found.
	TooManyMatches,
}

/// Reason why a game was automatically matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum AutomaticMatchReason {
	/// Matched by an alternative name which was exactly matching the title.
	AlternativeName,

	/// Matched by the direct name which was exactly matching the title.
	DirectName,

	/// A Game which is a clone of this game (a different version) was matched.
	ViaChild,

	/// A Game which this game is a clone of (a different version) was matched.
	ViaParent,
}

impl From<entity::signature_metadata_mapping::Model> for ExternalMetadata {
	fn from(value: entity::signature_metadata_mapping::Model) -> Self {
		ExternalMetadata {
			provider_name: value.provider.into(),
			provider_id: value.provider_id,
			match_type: value.match_type.into(),
			comment: value.comment,
			manual_match_type: value.manual_match_type.map(Into::into),
			failed_match_reason: value.failed_match_reason.map(Into::into),
			automatic_match_reason: value.automatic_match_reason.map(Into::into),
		}
	}
}

impl From<MetadataProviderEnum> for MetadataProvider {
	fn from(metadata_provider: MetadataProviderEnum) -> Self {
		match metadata_provider {
			MetadataProviderEnum::Igdb => MetadataProvider::IGDB,
		}
	}
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
			FailedMatchReasonEnum::NoDirectMatch => FailedMatchReason::NoDirectMatch,
			FailedMatchReasonEnum::TooManyMatches => FailedMatchReason::TooManyMatches,
		}
	}
}

impl From<AutomaticMatchReasonEnum> for AutomaticMatchReason {
	fn from(automatic_match_reason: AutomaticMatchReasonEnum) -> Self {
		match automatic_match_reason {
			AutomaticMatchReasonEnum::AlternativeName => AutomaticMatchReason::AlternativeName,
			AutomaticMatchReasonEnum::DirectName => AutomaticMatchReason::DirectName,
			AutomaticMatchReasonEnum::ViaChild => AutomaticMatchReason::ViaChild,
			AutomaticMatchReasonEnum::ViaParent => AutomaticMatchReason::ViaParent,
		}
	}
}
