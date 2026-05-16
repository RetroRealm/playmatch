use derive_builder::Builder;
use entity::sea_orm_active_enums::{
	AutomaticMatchReasonEnum, FailedMatchReasonEnum, ManualMatchModeEnum, MatchTypeEnum,
	MetadataProviderEnum,
};
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Response for a company including external metadata.
#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompanyMetadataResponse {
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
pub struct PlatformMetadataResponse {
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
	pub match_type: MetadataMatchType,

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
	/// SteamGridDB (https://www.steamgriddb.com/)
	SteamGridDB,
	/// ScreenScraper (https://www.screenscraper.fr/)
	ScreenScraper,
	/// MobyGames (https://www.mobygames.com/)
	MobyGames,
	/// LaunchBox Games Database (https://gamesdb.launchbox-app.com/)
	LaunchBox,
	/// EmuReady (https://www.emuready.com/)
	EmuReady,
	/// OpenVGDB (https://github.com/OpenVGDB/OpenVGDB)
	OpenVGDB,
	/// RetroAchievements (https://retroachievements.org/)
	RetroAchievements,
}

/// Match types for a game
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum MetadataMatchType {
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

	/// Game was manually matched by the community (via Discord as example).
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

	/// Multiple candidates tied at the top of the score; matcher refused to pick.
	Ambiguous,
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

	/// Matched by the normalized name (colons and dashes removed, Leading and trailing `The ` and `, The` removed, Leading and trailing `A ` and `An ` removed) matching the normalized title.
	NormalizedName,

	/// Matched by the normalized alternative name (colons and dashes removed, Leading and trailing `The ` and `, The` removed, Leading and trailing `A ` and `An ` removed) matching the normalized title.
	NormalizedAlternativeName,

	/// Matched by an MD5 hash of one of the game's files.
	Md5Hash,

	/// Matched by a SHA-1 hash of one of the game's files.
	Sha1Hash,

	/// Matched by a CRC32 of one of the game's files.
	CrcHash,

	/// Matched by another provider's canonical title via cross-provider name propagation, exact lower-case compare.
	CrossProviderDirectName,

	/// Matched by another provider's canonical title via cross-provider name propagation, after normalisation.
	CrossProviderNormalizedName,
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

impl From<ManualMatchMode> for ManualMatchModeEnum {
	fn from(value: ManualMatchMode) -> Self {
		match value {
			ManualMatchMode::Admin => ManualMatchModeEnum::Admin,
			ManualMatchMode::Community => ManualMatchModeEnum::Community,
			ManualMatchMode::Trusted => ManualMatchModeEnum::Trusted,
		}
	}
}

impl From<MetadataProviderEnum> for MetadataProvider {
	fn from(metadata_provider: MetadataProviderEnum) -> Self {
		match metadata_provider {
			MetadataProviderEnum::Igdb => MetadataProvider::IGDB,
			MetadataProviderEnum::Steamgriddb => MetadataProvider::SteamGridDB,
			MetadataProviderEnum::Screenscraper => MetadataProvider::ScreenScraper,
			MetadataProviderEnum::Mobygames => MetadataProvider::MobyGames,
			MetadataProviderEnum::Launchbox => MetadataProvider::LaunchBox,
			MetadataProviderEnum::EmuReady => MetadataProvider::EmuReady,
			MetadataProviderEnum::OpenVGDB => MetadataProvider::OpenVGDB,
			MetadataProviderEnum::RetroAchievements => MetadataProvider::RetroAchievements,
		}
	}
}

impl From<MetadataProvider> for MetadataProviderEnum {
	fn from(metadata_provider: MetadataProvider) -> Self {
		match metadata_provider {
			MetadataProvider::IGDB => MetadataProviderEnum::Igdb,
			MetadataProvider::SteamGridDB => MetadataProviderEnum::Steamgriddb,
			MetadataProvider::ScreenScraper => MetadataProviderEnum::Screenscraper,
			MetadataProvider::MobyGames => MetadataProviderEnum::Mobygames,
			MetadataProvider::LaunchBox => MetadataProviderEnum::Launchbox,
			MetadataProvider::EmuReady => MetadataProviderEnum::EmuReady,
			MetadataProvider::OpenVGDB => MetadataProviderEnum::OpenVGDB,
			MetadataProvider::RetroAchievements => MetadataProviderEnum::RetroAchievements,
		}
	}
}

impl From<MatchTypeEnum> for MetadataMatchType {
	fn from(match_type: MatchTypeEnum) -> Self {
		match match_type {
			MatchTypeEnum::Automatic => MetadataMatchType::Automatic,
			MatchTypeEnum::Failed => MetadataMatchType::Failed,
			MatchTypeEnum::Manual => MetadataMatchType::Manual,
			MatchTypeEnum::None => MetadataMatchType::None,
		}
	}
}

impl From<MetadataMatchType> for MatchTypeEnum {
	fn from(match_type: MetadataMatchType) -> Self {
		match match_type {
			MetadataMatchType::Automatic => MatchTypeEnum::Automatic,
			MetadataMatchType::Failed => MatchTypeEnum::Failed,
			MetadataMatchType::Manual => MatchTypeEnum::Manual,
			MetadataMatchType::None => MatchTypeEnum::None,
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
			FailedMatchReasonEnum::Ambiguous => FailedMatchReason::Ambiguous,
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
			AutomaticMatchReasonEnum::NormalizedName => AutomaticMatchReason::NormalizedName,
			AutomaticMatchReasonEnum::NormalizedAlternativeName => {
				AutomaticMatchReason::NormalizedAlternativeName
			}
			AutomaticMatchReasonEnum::Md5Hash => AutomaticMatchReason::Md5Hash,
			AutomaticMatchReasonEnum::Sha1Hash => AutomaticMatchReason::Sha1Hash,
			AutomaticMatchReasonEnum::CrcHash => AutomaticMatchReason::CrcHash,
			AutomaticMatchReasonEnum::CrossProviderDirectName => {
				AutomaticMatchReason::CrossProviderDirectName
			}
			AutomaticMatchReasonEnum::CrossProviderNormalizedName => {
				AutomaticMatchReason::CrossProviderNormalizedName
			}
		}
	}
}
