pub mod matching;
pub mod suggestion;
pub mod user;

use chrono::{DateTime, Utc};
use derive_builder::Builder;
use entity::sea_orm_active_enums::{
	AutomaticMatchReasonEnum, FailedMatchReasonEnum, ManualMatchModeEnum, MatchTypeEnum,
	MetadataProviderEnum,
};
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};
use strum::EnumIter;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, IntoParams)]
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

/// Result of a manual match.
#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdatedMatchResult {
	/// ID of the entity matched (game, platform or company).
	pub id: Uuid,

	/// The updated ExternalMetadata for the entity.
	pub external_metadata: ExternalMetadata,
}

/// Result of a game match, containing external metadata ids.
#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameMetadataMatchResult {
	/// The type of match that was found.
	pub game_match_type: GameMatchType,

	/// If a match was found, the ID of the matched game.
	pub id: Option<Uuid>,

	/// External metadata for the matched game.
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub external_metadata: Vec<ExternalMetadata>,
}

/// Result of a game match including company, platform and files.
#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameAndRelationMatchResult {
	/// The type of match that was found.
	pub game_match_type: GameMatchType,

	/// If a match was found, the game found.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub game: Option<PlaymatchGame>,

	/// If a match was found, the game files for this game.
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub game_files: Vec<PlaymatchGameFile>,

	/// If a match was found and a company for this platform exists, the company.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub company: Option<PlaymatchCompany>,

	/// If a match was found, the platform for this game.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub platform: Option<PlaymatchPlatform>,

	/// if a match was found, the signature group who published the dat file this game belongs to.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub signature_group: Option<PlaymatchSignatureGroup>,

	/// if a match was found, the dat file this game belongs to.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub dat_file: Option<PlaymatchDatFile>,

	/// if a match was found, the dat file import this game belongs to.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub dat_file_import: Option<PlaymatchDatFileImport>,

	/// If a match was found, External metadata for the game.
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub external_metadata: Vec<ExternalMetadata>,
}

/// Result of a game match including company, platform and files.
#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameAndRelationsResult {
	/// the game found.
	pub game: PlaymatchGame,

	/// If a match was found, the game files for this game.
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub game_files: Vec<PlaymatchGameFile>,

	/// If a match was found and a company for this platform exists, the company.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub company: Option<PlaymatchCompany>,

	/// If a match was found, the platform for this game.
	pub platform: PlaymatchPlatform,

	/// if a match was found, the signature group who published the dat file this game belongs to.
	pub signature_group: PlaymatchSignatureGroup,

	/// if a match was found, the dat file this game belongs to.
	pub dat_file: PlaymatchDatFile,

	/// if a match was found, the dat file import this game belongs to.
	pub dat_file_import: PlaymatchDatFileImport,
}

/// contains basic information about an imported dat file in Playmatch.
#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
pub struct PlaymatchDatFileImport {
	/// The ID of the dat file import.
	pub id: Uuid,

	/// The ID of the dat file this import belongs to.
	pub dat_file_id: Uuid,

	/// The name of the imported file, this contains usually some information like version and date of creation
	pub name: String,

	/// The version of the dat file which was imported.
	pub version: String,

	/// MD5 hash of the imported dat file.
	pub md5: String,

	/// When the dat file was imported into playmatch.
	pub imported_at: DateTime<Utc>,

	/// When the dat file import was created inside playmatch.
	pub created_at: DateTime<Utc>,

	/// When the dat file import was last updated inside playmatch.
	pub updated_at: DateTime<Utc>,
}

/// contains basic information about a Dat File.
#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
pub struct PlaymatchDatFile {
	/// The ID of the dat file.
	pub id: Uuid,

	/// The name of the dat file.
	pub name: String,

	/// Optional company for the platform this dat file is for.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub company_id: Option<Uuid>,

	/// The platform this dat file is for.
	pub platform_id: Uuid,

	/// The current version of the dat file.
	pub current_version: String,

	/// The id of the signature group which publishes this dat file.
	pub signature_group_id: Uuid,

	/// Optional tags for the dat file.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tags: Option<Vec<String>>,

	/// The subset this dat file is for, if any
	#[serde(skip_serializing_if = "Option::is_none")]
	pub subset: Option<String>,

	/// When the dat file was created inside playmatch.
	pub created_at: DateTime<Utc>,

	/// When the dat file was last updated inside playmatch.
	pub updated_at: DateTime<Utc>,
}

/// contains basic information about a Signature Group.
#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
pub struct PlaymatchSignatureGroup {
	/// The ID of the signature group.
	pub id: Uuid,

	/// The ID of the dat file import this signature group belongs to.
	pub name: String,

	/// Optional Link to the website of the signature group.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub website_link: Option<String>,

	/// Optional description of the signature group.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,

	/// When the signature group was created inside playmatch.
	pub created_at: DateTime<Utc>,

	/// When the signature group was last updated inside playmatch.
	pub updated_at: DateTime<Utc>,
}

/// contains basic information about a Platform.
#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
pub struct PlaymatchPlatform {
	/// The ID of the platform.
	pub id: Uuid,

	/// The name of the platform.
	pub name: String,

	/// Optional id of the company that made the platform.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub company_id: Option<Uuid>,

	/// When the platform was last updated inside playmatch.
	pub updated_at: DateTime<Utc>,

	/// When the platform was created inside playmatch.
	pub created_at: DateTime<Utc>,
}

/// contains basic information about a Company.
#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
pub struct PlaymatchCompany {
	/// The ID of the company.
	pub id: Uuid,

	/// The name of the company.
	pub name: String,

	/// When the company was last updated inside playmatch.
	pub updated_at: DateTime<Utc>,

	/// When the company was created inside playmatch.
	pub created_at: DateTime<Utc>,
}

/// contains basic information about a Game.
#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
pub struct PlaymatchGame {
	/// The ID of the game.
	pub id: Uuid,

	/// The name of the game.
	pub name: String,

	/// Optional description of the game.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,

	/// Optional categories for the game.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub categories: Option<Vec<String>>,

	/// Optional which game this game is a clone of (different editions/versions).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub clone_of: Option<Uuid>,

	/// When the game was created inside playmatch.
	pub created_at: DateTime<Utc>,

	/// When the game was last updated inside playmatch.
	pub updated_at: DateTime<Utc>,
}

/// contains basic information about a Game File.
#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
pub struct PlaymatchGameFile {
	/// The ID of the game file.
	pub id: Uuid,

	/// The ID of the game this file belongs to.
	pub game_id: Uuid,

	/// The name of the file, including extension.
	pub file_name: String,

	/// The size of the file in bytes, if available.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub file_size_in_bytes: Option<i64>,

	/// Optional crc32 checksum of the file.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub crc: Option<String>,

	/// Optional MD5 hash of the file.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub md5: Option<String>,

	/// Optional SHA1 hash of the file.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub sha1: Option<String>,

	/// Optional SHA256 hash of the file.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub sha256: Option<String>,

	/// Optional status of the file, e.g. "verified", etc.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub status: Option<String>,

	/// Optional serial number of the rom file, if applicable.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub serial: Option<String>,

	/// When the game file was created inside playmatch.
	pub created_at: DateTime<Utc>,

	/// When the game file was last updated inside playmatch.
	pub updated_at: DateTime<Utc>,
}

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
}

impl From<entity::signature_group::Model> for PlaymatchSignatureGroup {
	fn from(value: entity::signature_group::Model) -> Self {
		PlaymatchSignatureGroup {
			id: value.id,
			name: value.name,
			website_link: value.website_link,
			description: value.description,
			created_at: value.created_at.into(),
			updated_at: value.updated_at.into(),
		}
	}
}

impl From<entity::dat_file::Model> for PlaymatchDatFile {
	fn from(value: entity::dat_file::Model) -> Self {
		PlaymatchDatFile {
			id: value.id,
			name: value.name,
			company_id: value.company_id,
			platform_id: value.platform_id,
			current_version: value.current_version,
			signature_group_id: value.signature_group_id,
			tags: value.tags,
			subset: value.subset,
			created_at: value.created_at.into(),
			updated_at: value.updated_at.into(),
		}
	}
}

impl From<entity::dat_file_import::Model> for PlaymatchDatFileImport {
	fn from(value: entity::dat_file_import::Model) -> Self {
		PlaymatchDatFileImport {
			id: value.id,
			dat_file_id: value.dat_file_id,
			name: value.name,
			version: value.version,
			md5: value.md5,
			imported_at: value.imported_at.into(),
			created_at: value.created_at.into(),
			updated_at: value.updated_at.into(),
		}
	}
}

impl From<entity::game::Model> for PlaymatchGame {
	fn from(value: entity::game::Model) -> Self {
		PlaymatchGame {
			id: value.id,
			name: value.name,
			description: value.description,
			categories: value.categories,
			clone_of: value.clone_of,
			created_at: value.created_at.into(),
			updated_at: value.updated_at.into(),
		}
	}
}

impl From<entity::platform::Model> for PlaymatchPlatform {
	fn from(value: entity::platform::Model) -> Self {
		PlaymatchPlatform {
			id: value.id,
			name: value.name,
			company_id: value.company_id,
			created_at: value.created_at.into(),
			updated_at: value.updated_at.into(),
		}
	}
}

impl From<entity::company::Model> for PlaymatchCompany {
	fn from(value: entity::company::Model) -> Self {
		PlaymatchCompany {
			id: value.id,
			name: value.name,
			created_at: value.created_at.into(),
			updated_at: value.updated_at.into(),
		}
	}
}

impl From<entity::game_file::Model> for PlaymatchGameFile {
	fn from(value: entity::game_file::Model) -> Self {
		PlaymatchGameFile {
			id: value.id,
			game_id: value.game_id,
			file_name: value.file_name,
			file_size_in_bytes: value.file_size_in_bytes,
			crc: value.crc,
			md5: value.md5,
			sha1: value.sha1,
			sha256: value.sha256,
			status: value.status,
			serial: value.serial,
			created_at: value.created_at.into(),
			updated_at: value.updated_at.into(),
		}
	}
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
		}
	}
}

impl From<MetadataProvider> for MetadataProviderEnum {
	fn from(metadata_provider: MetadataProvider) -> Self {
		match metadata_provider {
			MetadataProvider::IGDB => MetadataProviderEnum::Igdb,
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

impl From<MatchType> for MatchTypeEnum {
	fn from(match_type: MatchType) -> Self {
		match match_type {
			MatchType::Automatic => MatchTypeEnum::Automatic,
			MatchType::Failed => MatchTypeEnum::Failed,
			MatchType::Manual => MatchTypeEnum::Manual,
			MatchType::None => MatchTypeEnum::None,
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
			AutomaticMatchReasonEnum::NormalizedName => AutomaticMatchReason::NormalizedName,
			AutomaticMatchReasonEnum::NormalizedAlternativeName => {
				AutomaticMatchReason::NormalizedAlternativeName
			}
		}
	}
}
