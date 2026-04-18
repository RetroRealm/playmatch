use crate::model::metadata::ExternalMetadata;
use crate::model::playmatch::{
	PlaymatchCompany, PlaymatchDatFile, PlaymatchDatFileImport, PlaymatchGame, PlaymatchGameFile,
	PlaymatchPlatform, PlaymatchSignatureGroup,
};
use derive_builder::Builder;
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

impl GameMatchType {
	pub fn metric_label(&self) -> &'static str {
		match self {
			GameMatchType::SHA256 => "sha256",
			GameMatchType::SHA1 => "sha1",
			GameMatchType::MD5 => "md5",
			GameMatchType::FileNameAndSize => "filename",
			GameMatchType::NoMatch => "no_match",
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
}
