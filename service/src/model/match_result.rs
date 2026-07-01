use crate::model::metadata::{ExternalMetadata, ExternalMetadataV2};
use crate::model::playmatch::{
	PlaymatchCompany, PlaymatchCompanyV2, PlaymatchDatFile, PlaymatchDatFileImport,
	PlaymatchDatFileImportV2, PlaymatchDatFileV2, PlaymatchGame, PlaymatchGameFile,
	PlaymatchGameFileV2, PlaymatchGameV2, PlaymatchPlatform, PlaymatchPlatformV2,
	PlaymatchSignatureGroup, PlaymatchSignatureGroupV2,
};
use crate::model::{MAX_NAME_INPUT_LEN, validate_optional_hex};
use derive_builder::Builder;
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};
use strum::EnumIter;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, IntoParams, ToSchema)]
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

	/// Optional CRC32 checksum of the game file.
	pub crc: Option<String>,
}

impl GameFileMatchSearch {
	pub fn validate(&self) -> Result<(), String> {
		if self.file_name.chars().count() > MAX_NAME_INPUT_LEN {
			return Err(format!("file_name exceeds {MAX_NAME_INPUT_LEN} characters"));
		}
		if self.file_size < 0 {
			return Err("file_size must be non-negative".to_string());
		}
		validate_optional_hex(&self.md5, 32, "md5")?;
		validate_optional_hex(&self.sha1, 40, "sha1")?;
		validate_optional_hex(&self.sha256, 64, "sha256")?;
		validate_optional_hex(&self.crc, 8, "crc")?;
		Ok(())
	}
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

	/// Matched by CRC32 checksum.
	CRC,

	/// Matched by file name and size.
	FileNameAndSize,

	/// No match found.
	NoMatch,
}

impl GameMatchType {
	pub fn metric_label(&self) -> &'static str {
		match self {
			GameMatchType::SHA256 => "sha256",
			GameMatchType::SHA1 => "sha1",
			GameMatchType::MD5 => "md5",
			GameMatchType::CRC => "crc",
			GameMatchType::FileNameAndSize => "filename",
			GameMatchType::NoMatch => "no_match",
		}
	}

	/// Returns the identify cache namespace segment for this match type, or
	/// `None` for [`GameMatchType::NoMatch`] (not cached).
	///
	/// Note: `FileNameAndSize` returns `"filename_size"` here while
	/// [`Self::metric_label`] returns `"filename"`. Cache hit/miss metrics
	/// use `"filename_size"` and identify attempt metrics use `"filename"`;
	/// the two segments stay distinct to keep existing Prometheus series
	/// stable.
	pub fn cache_segment(&self) -> Option<&'static str> {
		match self {
			GameMatchType::SHA256 => Some("sha256"),
			GameMatchType::SHA1 => Some("sha1"),
			GameMatchType::MD5 => Some("md5"),
			GameMatchType::CRC => Some("crc"),
			GameMatchType::FileNameAndSize => Some("filename_size"),
			GameMatchType::NoMatch => None,
		}
	}
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

	/// External metadata mappings for the game (one row per matched provider).
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub external_metadata: Vec<ExternalMetadata>,
}

/// The V2 identify outcome: the primary match and its related records, plus the
/// co-hashed siblings surfaced in `additionalMatches`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameAndRelationMatchResultV2 {
	/// The type of match that was found.
	pub game_match_type: GameMatchType,

	/// If a match was found, the game found.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub game: Option<PlaymatchGameV2>,

	/// If a match was found, the game files for this game.
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub game_files: Vec<PlaymatchGameFileV2>,

	/// If a match was found and a company for this platform exists, the company.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub company: Option<PlaymatchCompanyV2>,

	/// If a match was found, the platform for this game.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub platform: Option<PlaymatchPlatformV2>,

	/// If a match was found, the signature group who published the dat file this game belongs to.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub signature_group: Option<PlaymatchSignatureGroupV2>,

	/// If a match was found, the dat file this game belongs to.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub dat_file: Option<PlaymatchDatFileV2>,

	/// If a match was found, the dat file import this game belongs to.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub dat_file_import: Option<PlaymatchDatFileImportV2>,

	/// If a match was found, External metadata for the game.
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub external_metadata: Vec<ExternalMetadataV2>,

	/// Co-hashed sibling games that share a content hash with the primary game,
	/// ranked after it. Empty on the V1-derived path and whenever the hash
	/// resolves to a single game.
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub additional_matches: Vec<GameAndRelationsResultV2>,
}

impl From<GameAndRelationMatchResult> for GameAndRelationMatchResultV2 {
	fn from(value: GameAndRelationMatchResult) -> Self {
		GameAndRelationMatchResultV2 {
			game_match_type: value.game_match_type,
			game: value.game.map(Into::into),
			game_files: value.game_files.into_iter().map(Into::into).collect(),
			company: value.company.map(Into::into),
			platform: value.platform.map(Into::into),
			signature_group: value.signature_group.map(Into::into),
			dat_file: value.dat_file.map(Into::into),
			dat_file_import: value.dat_file_import.map(Into::into),
			external_metadata: value
				.external_metadata
				.into_iter()
				.map(Into::into)
				.collect(),
			additional_matches: Vec::new(),
		}
	}
}

/// A single co-hashed game and its related records, as carried in the V2
/// `additionalMatches` array. The game is always present; the spine records
/// below resolve from it.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameAndRelationsResultV2 {
	/// The game found.
	pub game: PlaymatchGameV2,

	/// The game files for this game.
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub game_files: Vec<PlaymatchGameFileV2>,

	/// The company for this platform, when one exists.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub company: Option<PlaymatchCompanyV2>,

	/// The platform for this game.
	pub platform: PlaymatchPlatformV2,

	/// The signature group who published the dat file this game belongs to.
	pub signature_group: PlaymatchSignatureGroupV2,

	/// The dat file this game belongs to.
	pub dat_file: PlaymatchDatFileV2,

	/// The dat file import this game belongs to.
	pub dat_file_import: PlaymatchDatFileImportV2,

	/// External metadata mappings for the game (one row per matched provider).
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub external_metadata: Vec<ExternalMetadataV2>,
}

impl From<GameAndRelationsResult> for GameAndRelationsResultV2 {
	fn from(value: GameAndRelationsResult) -> Self {
		GameAndRelationsResultV2 {
			game: value.game.into(),
			game_files: value.game_files.into_iter().map(Into::into).collect(),
			company: value.company.map(Into::into),
			platform: value.platform.into(),
			signature_group: value.signature_group.into(),
			dat_file: value.dat_file.into(),
			dat_file_import: value.dat_file_import.into(),
			external_metadata: value
				.external_metadata
				.into_iter()
				.map(Into::into)
				.collect(),
		}
	}
}
