use chrono::{DateTime, Utc};
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

	/// The name of the company that made the platform, absent when unknown.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub company_name: Option<String>,

	/// The id of the company that made the platform, absent when unknown.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub company_id: Option<Uuid>,

	/// External metadata for the platform.
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub external_metadata: Vec<ExternalMetadata>,
}

/// Response for a game including external metadata.
#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameMetadataResponse {
	/// The ID of the game.
	pub id: Uuid,

	/// The name of the game.
	pub name: String,

	/// A description of the game, absent when the dat provides none.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,

	/// Categories for the game, absent when the dat provides none.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub categories: Option<Vec<String>>,

	/// The id of the game this game is a clone of (a different edition or regional version), absent when the game is not a clone.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub clone_of: Option<Uuid>,

	/// When the game was created inside Playmatch.
	pub created_at: DateTime<Utc>,

	/// When the game was last updated inside Playmatch.
	pub updated_at: DateTime<Utc>,

	/// External metadata mappings for the game (one row per matched provider).
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub external_metadata: Vec<ExternalMetadata>,
}

/// External metadata for a game/platform/company.
#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExternalMetadata {
	/// The name of the metadata provider.
	pub provider_name: MetadataProvider,

	/// The id of the game, company, or platform on the metadata provider, absent when there is no match.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub provider_id: Option<String>,

	/// How this game was matched to this provider.
	pub match_type: MetadataMatchType,

	/// A comment about the match, absent when there is none.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub comment: Option<String>,

	/// The type of manual match, absent when the match is not manual.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub manual_match_type: Option<ManualMatchMode>,

	/// Why the automatic match failed, absent when the match did not fail.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub failed_match_reason: Option<FailedMatchReason>,

	/// The reason for an automatic match, absent when the match was not automatic or when the reason has no v1 representation.
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
	/// TheGamesDB (https://thegamesdb.net/)
	TheGamesDB,
	/// Hasheous (https://hasheous.org/)
	Hasheous,
}

/// How a game, platform or company was matched to a metadata provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum MetadataMatchType {
	/// The game was matched automatically, without human input.
	Automatic,

	/// Automatic game matching failed and no manual match was done.
	Failed,

	/// The game was matched by hand; `manualMatchType` records the trust level of the match.
	Manual,

	/// No match was done.
	None,
}

/// How a game was manually matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum ManualMatchMode {
	/// Matched by an Admin user; the most trusted manual match.
	Admin,

	/// Matched by the community, for example through an approved suggestion.
	Community,

	/// Matched by a user with Trusted permissions.
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

	/// Skipped because the game has more files than the provider's catalogue
	/// indexes (multi-track arcade dumps, per-disc-segment images).
	TooManyFiles,
}

/// Reason why a game was automatically matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum AutomaticMatchReason {
	/// Matched by an alternative name exactly matching the title.
	AlternativeName,

	/// Matched by the game's direct name exactly matching the title.
	DirectName,

	/// A game which is a clone of this game (a different version) was matched.
	ViaChild,

	/// A game which this game is a clone of (a different version) was matched.
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

	/// Matched by another provider's canonical title via cross-provider name propagation, after normalization.
	CrossProviderNormalizedName,
}

/// External metadata for a game/platform/company.
#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExternalMetadataV2 {
	/// The name of the metadata provider.
	pub provider_name: MetadataProvider,

	/// The id of the game, company, or platform on the metadata provider, absent when there is no match.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub provider_id: Option<String>,

	/// How this game was matched to this provider.
	pub match_type: MetadataMatchType,

	/// A comment about the match, absent when there is none.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub comment: Option<String>,

	/// The type of manual match, absent when the match is not manual.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub manual_match_type: Option<ManualMatchMode>,

	/// Why the automatic match failed, absent when the match did not fail.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub failed_match_reason: Option<FailedMatchReason>,

	/// The reason for an automatic match, absent when the match was not automatic.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub automatic_match_reason: Option<AutomaticMatchReasonV2>,
}

/// Reason why a game was automatically matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum AutomaticMatchReasonV2 {
	/// Matched by an alternative name exactly matching the title.
	AlternativeName,

	/// Matched by the game's direct name exactly matching the title.
	DirectName,

	/// A game which is a clone of this game (a different version) was matched.
	ViaChild,

	/// A game which this game is a clone of (a different version) was matched.
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

	/// Matched by another provider's canonical title via cross-provider name propagation, after normalization.
	CrossProviderNormalizedName,

	/// Propagated from a content-sibling game that shares the same file hash set.
	ViaContentHash,

	/// Matched by a SHA-256 hash of one of the game's files.
	Sha256Hash,
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
			automatic_match_reason: value
				.automatic_match_reason
				.and_then(AutomaticMatchReason::from_entity),
		}
	}
}

impl From<ExternalMetadata> for ExternalMetadataV2 {
	fn from(value: ExternalMetadata) -> Self {
		ExternalMetadataV2 {
			provider_name: value.provider_name,
			provider_id: value.provider_id,
			match_type: value.match_type,
			comment: value.comment,
			manual_match_type: value.manual_match_type,
			failed_match_reason: value.failed_match_reason,
			automatic_match_reason: value.automatic_match_reason.map(Into::into),
		}
	}
}

impl From<AutomaticMatchReason> for AutomaticMatchReasonV2 {
	fn from(value: AutomaticMatchReason) -> Self {
		match value {
			AutomaticMatchReason::AlternativeName => AutomaticMatchReasonV2::AlternativeName,
			AutomaticMatchReason::DirectName => AutomaticMatchReasonV2::DirectName,
			AutomaticMatchReason::ViaChild => AutomaticMatchReasonV2::ViaChild,
			AutomaticMatchReason::ViaParent => AutomaticMatchReasonV2::ViaParent,
			AutomaticMatchReason::NormalizedName => AutomaticMatchReasonV2::NormalizedName,
			AutomaticMatchReason::NormalizedAlternativeName => {
				AutomaticMatchReasonV2::NormalizedAlternativeName
			}
			AutomaticMatchReason::Md5Hash => AutomaticMatchReasonV2::Md5Hash,
			AutomaticMatchReason::Sha1Hash => AutomaticMatchReasonV2::Sha1Hash,
			AutomaticMatchReason::CrcHash => AutomaticMatchReasonV2::CrcHash,
			AutomaticMatchReason::CrossProviderDirectName => {
				AutomaticMatchReasonV2::CrossProviderDirectName
			}
			AutomaticMatchReason::CrossProviderNormalizedName => {
				AutomaticMatchReasonV2::CrossProviderNormalizedName
			}
		}
	}
}

impl From<entity::signature_metadata_mapping::Model> for ExternalMetadataV2 {
	fn from(value: entity::signature_metadata_mapping::Model) -> Self {
		ExternalMetadataV2 {
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
			MetadataProviderEnum::TheGamesDB => MetadataProvider::TheGamesDB,
			MetadataProviderEnum::Hasheous => MetadataProvider::Hasheous,
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
			MetadataProvider::TheGamesDB => MetadataProviderEnum::TheGamesDB,
			MetadataProvider::Hasheous => MetadataProviderEnum::Hasheous,
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
			FailedMatchReasonEnum::TooManyFiles => FailedMatchReason::TooManyFiles,
		}
	}
}

impl AutomaticMatchReason {
	/// Maps a stored match reason onto the v1 enum, returning `None` for reasons
	/// that have no v1 representation. The v1 surface omits the field for those
	/// rows rather than emitting a value outside its schema.
	fn from_entity(value: AutomaticMatchReasonEnum) -> Option<Self> {
		match value {
			AutomaticMatchReasonEnum::AlternativeName => {
				Some(AutomaticMatchReason::AlternativeName)
			}
			AutomaticMatchReasonEnum::DirectName => Some(AutomaticMatchReason::DirectName),
			AutomaticMatchReasonEnum::ViaChild => Some(AutomaticMatchReason::ViaChild),
			AutomaticMatchReasonEnum::ViaParent => Some(AutomaticMatchReason::ViaParent),
			AutomaticMatchReasonEnum::NormalizedName => Some(AutomaticMatchReason::NormalizedName),
			AutomaticMatchReasonEnum::NormalizedAlternativeName => {
				Some(AutomaticMatchReason::NormalizedAlternativeName)
			}
			AutomaticMatchReasonEnum::Md5Hash => Some(AutomaticMatchReason::Md5Hash),
			AutomaticMatchReasonEnum::Sha1Hash => Some(AutomaticMatchReason::Sha1Hash),
			AutomaticMatchReasonEnum::CrcHash => Some(AutomaticMatchReason::CrcHash),
			AutomaticMatchReasonEnum::CrossProviderDirectName => {
				Some(AutomaticMatchReason::CrossProviderDirectName)
			}
			AutomaticMatchReasonEnum::CrossProviderNormalizedName => {
				Some(AutomaticMatchReason::CrossProviderNormalizedName)
			}
			AutomaticMatchReasonEnum::ViaContentHash | AutomaticMatchReasonEnum::Sha256Hash => None,
		}
	}
}

impl From<AutomaticMatchReasonEnum> for AutomaticMatchReasonV2 {
	fn from(automatic_match_reason: AutomaticMatchReasonEnum) -> Self {
		match automatic_match_reason {
			AutomaticMatchReasonEnum::AlternativeName => AutomaticMatchReasonV2::AlternativeName,
			AutomaticMatchReasonEnum::DirectName => AutomaticMatchReasonV2::DirectName,
			AutomaticMatchReasonEnum::ViaChild => AutomaticMatchReasonV2::ViaChild,
			AutomaticMatchReasonEnum::ViaParent => AutomaticMatchReasonV2::ViaParent,
			AutomaticMatchReasonEnum::NormalizedName => AutomaticMatchReasonV2::NormalizedName,
			AutomaticMatchReasonEnum::NormalizedAlternativeName => {
				AutomaticMatchReasonV2::NormalizedAlternativeName
			}
			AutomaticMatchReasonEnum::Md5Hash => AutomaticMatchReasonV2::Md5Hash,
			AutomaticMatchReasonEnum::Sha1Hash => AutomaticMatchReasonV2::Sha1Hash,
			AutomaticMatchReasonEnum::CrcHash => AutomaticMatchReasonV2::CrcHash,
			AutomaticMatchReasonEnum::CrossProviderDirectName => {
				AutomaticMatchReasonV2::CrossProviderDirectName
			}
			AutomaticMatchReasonEnum::CrossProviderNormalizedName => {
				AutomaticMatchReasonV2::CrossProviderNormalizedName
			}
			AutomaticMatchReasonEnum::ViaContentHash => AutomaticMatchReasonV2::ViaContentHash,
			AutomaticMatchReasonEnum::Sha256Hash => AutomaticMatchReasonV2::Sha256Hash,
		}
	}
}
