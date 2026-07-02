use chrono::{DateTime, Utc};
use derive_builder::Builder;
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A single import run of a dat file, returned by the import-history lookups and embedded in identify and game responses.
#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
pub struct PlaymatchDatFileImport {
	/// The ID of the dat file import.
	pub id: Uuid,

	/// The ID of the dat file this import belongs to.
	pub dat_file_id: Uuid,

	/// The name of the imported file, which usually encodes the version and build date.
	pub name: String,

	/// The version of the dat file which was imported.
	pub version: String,

	/// The MD5 hash of the imported dat file.
	pub md5: String,

	/// When the dat file was imported into Playmatch.
	pub imported_at: DateTime<Utc>,

	/// When the dat file import was created inside Playmatch.
	pub created_at: DateTime<Utc>,

	/// When the dat file import was last updated inside Playmatch.
	pub updated_at: DateTime<Utc>,
}

/// A dat file tracked by Playmatch, embedded in identify and game responses.
#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
pub struct PlaymatchDatFile {
	/// The ID of the dat file.
	pub id: Uuid,

	/// The name of the dat file.
	pub name: String,

	/// The id of the company for the platform this dat file is for, absent when the platform has no company.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub company_id: Option<Uuid>,

	/// The id of the platform this dat file is for.
	pub platform_id: Uuid,

	/// The current version of the dat file.
	pub current_version: String,

	/// The id of the signature group which publishes this dat file.
	pub signature_group_id: Uuid,

	/// Tags for the dat file, absent when it has none.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tags: Option<Vec<String>>,

	/// The subset this dat file is for, absent when it has none.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub subset: Option<String>,

	/// When the dat file was created inside Playmatch.
	pub created_at: DateTime<Utc>,

	/// When the dat file was last updated inside Playmatch.
	pub updated_at: DateTime<Utc>,
}

/// A signature group, the project that publishes dat files, returned by the signature group endpoints and embedded in identify and game responses.
#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
pub struct PlaymatchSignatureGroup {
	/// The ID of the signature group.
	pub id: Uuid,

	/// The name of the signature group, for example No-Intro or Redump.
	pub name: String,

	/// The signature group's website URL, absent when unknown.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub website_link: Option<String>,

	/// A description of the signature group, absent when it has none.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,

	/// When the signature group was created inside Playmatch.
	pub created_at: DateTime<Utc>,

	/// When the signature group was last updated inside Playmatch.
	pub updated_at: DateTime<Utc>,
}

/// A platform in the Playmatch catalogue, embedded in identify and game responses.
#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
pub struct PlaymatchPlatform {
	/// The ID of the platform.
	pub id: Uuid,

	/// The name of the platform.
	pub name: String,

	/// The id of the company that made the platform, absent when unknown.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub company_id: Option<Uuid>,

	/// When the platform was last updated inside Playmatch.
	pub updated_at: DateTime<Utc>,

	/// When the platform was created inside Playmatch.
	pub created_at: DateTime<Utc>,
}

/// A company in the Playmatch catalogue, embedded in identify and game responses.
#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
pub struct PlaymatchCompany {
	/// The ID of the company.
	pub id: Uuid,

	/// The name of the company.
	pub name: String,

	/// When the company was last updated inside Playmatch.
	pub updated_at: DateTime<Utc>,

	/// When the company was created inside Playmatch.
	pub created_at: DateTime<Utc>,
}

/// A game in the Playmatch catalogue, embedded in identify and game responses.
#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
pub struct PlaymatchGame {
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

	/// Whether this game is still present in the current version of its dat file.
	pub current_in_latest_dat: bool,

	/// The version of the last dat file release this game was seen in, absent when unknown.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub last_seen_dat_version: Option<String>,

	/// The id of the last dat file import this game was seen in, absent when unknown.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub last_seen_dat_file_import_id: Option<Uuid>,

	/// When the game was created inside Playmatch.
	pub created_at: DateTime<Utc>,

	/// When the game was last updated inside Playmatch.
	pub updated_at: DateTime<Utc>,
}

/// A single file (ROM entry) of a game, embedded in identify and game responses.
#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
pub struct PlaymatchGameFile {
	/// The ID of the game file.
	pub id: Uuid,

	/// The ID of the game this file belongs to.
	pub game_id: Uuid,

	/// The name of the file, including extension.
	pub file_name: String,

	/// The size of the file in bytes, absent when the dat provides none.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub file_size_in_bytes: Option<i64>,

	/// The CRC32 checksum of the file, absent when the dat provides none.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub crc: Option<String>,

	/// The MD5 hash of the file, absent when the dat provides none.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub md5: Option<String>,

	/// The SHA1 hash of the file, absent when the dat provides none.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub sha1: Option<String>,

	/// The SHA256 hash of the file, absent when the dat provides none.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub sha256: Option<String>,

	/// The dat-file status of the file, for example `verified`, absent when the dat provides none.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub status: Option<String>,

	/// The serial number of the ROM, absent when the dat provides none.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub serial: Option<String>,

	/// Whether this hash is still present in the current version of its dat file.
	pub current_in_latest_dat: bool,

	/// The version of the last dat file release this hash was seen in, absent when unknown.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub last_seen_dat_version: Option<String>,

	/// The id of the last dat file import this hash was seen in, absent when unknown.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub last_seen_dat_file_import_id: Option<Uuid>,

	/// When the game file was created inside Playmatch.
	pub created_at: DateTime<Utc>,

	/// When the game file was last updated inside Playmatch.
	pub updated_at: DateTime<Utc>,
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
			current_in_latest_dat: value.is_current,
			last_seen_dat_version: None,
			last_seen_dat_file_import_id: value.last_seen_dat_file_import_id,
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
			current_in_latest_dat: value.is_current,
			last_seen_dat_version: None,
			last_seen_dat_file_import_id: value.last_seen_dat_file_import_id,
			created_at: value.created_at.into(),
			updated_at: value.updated_at.into(),
		}
	}
}

/// A single candidate from a fuzzy game-name search. Carries just enough to let
/// a caller pick a result and follow up with a get-game call by id.
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct GameNameSearchResult {
	/// The ID of the game.
	pub id: Uuid,

	/// The name of the game.
	pub name: String,

	/// The ID of the platform this game belongs to.
	pub platform_id: Uuid,

	/// The name of the platform this game belongs to.
	pub platform_name: String,
}

/// A single import run of a dat file, returned by the import-history lookups and embedded in identify and game responses.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlaymatchDatFileImportV2 {
	/// The ID of the dat file import.
	pub id: Uuid,

	/// The ID of the dat file this import belongs to.
	pub dat_file_id: Uuid,

	/// The name of the imported file, which usually encodes the version and build date.
	pub name: String,

	/// The version of the dat file which was imported.
	pub version: String,

	/// The MD5 hash of the imported dat file.
	pub md5: String,

	/// When the dat file was imported into Playmatch.
	pub imported_at: DateTime<Utc>,

	/// When the dat file import was created inside Playmatch.
	pub created_at: DateTime<Utc>,

	/// When the dat file import was last updated inside Playmatch.
	pub updated_at: DateTime<Utc>,
}

impl From<PlaymatchDatFileImport> for PlaymatchDatFileImportV2 {
	fn from(value: PlaymatchDatFileImport) -> Self {
		PlaymatchDatFileImportV2 {
			id: value.id,
			dat_file_id: value.dat_file_id,
			name: value.name,
			version: value.version,
			md5: value.md5,
			imported_at: value.imported_at,
			created_at: value.created_at,
			updated_at: value.updated_at,
		}
	}
}

/// A dat file tracked by Playmatch, embedded in identify and game responses.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlaymatchDatFileV2 {
	/// The ID of the dat file.
	pub id: Uuid,

	/// The name of the dat file.
	pub name: String,

	/// The id of the company for the platform this dat file is for, absent when the platform has no company.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub company_id: Option<Uuid>,

	/// The id of the platform this dat file is for.
	pub platform_id: Uuid,

	/// The current version of the dat file.
	pub current_version: String,

	/// The id of the signature group which publishes this dat file.
	pub signature_group_id: Uuid,

	/// Tags for the dat file, absent when it has none.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tags: Option<Vec<String>>,

	/// The subset this dat file is for, absent when it has none.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub subset: Option<String>,

	/// When the dat file was created inside Playmatch.
	pub created_at: DateTime<Utc>,

	/// When the dat file was last updated inside Playmatch.
	pub updated_at: DateTime<Utc>,
}

impl From<PlaymatchDatFile> for PlaymatchDatFileV2 {
	fn from(value: PlaymatchDatFile) -> Self {
		PlaymatchDatFileV2 {
			id: value.id,
			name: value.name,
			company_id: value.company_id,
			platform_id: value.platform_id,
			current_version: value.current_version,
			signature_group_id: value.signature_group_id,
			tags: value.tags,
			subset: value.subset,
			created_at: value.created_at,
			updated_at: value.updated_at,
		}
	}
}

/// A signature group, the project that publishes dat files, returned by the signature group endpoints and embedded in identify and game responses.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlaymatchSignatureGroupV2 {
	/// The ID of the signature group.
	pub id: Uuid,

	/// The name of the signature group, for example No-Intro or Redump.
	pub name: String,

	/// The signature group's website URL, absent when unknown.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub website_link: Option<String>,

	/// A description of the signature group, absent when it has none.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,

	/// When the signature group was created inside Playmatch.
	pub created_at: DateTime<Utc>,

	/// When the signature group was last updated inside Playmatch.
	pub updated_at: DateTime<Utc>,
}

impl From<PlaymatchSignatureGroup> for PlaymatchSignatureGroupV2 {
	fn from(value: PlaymatchSignatureGroup) -> Self {
		PlaymatchSignatureGroupV2 {
			id: value.id,
			name: value.name,
			website_link: value.website_link,
			description: value.description,
			created_at: value.created_at,
			updated_at: value.updated_at,
		}
	}
}

impl From<entity::signature_group::Model> for PlaymatchSignatureGroupV2 {
	fn from(value: entity::signature_group::Model) -> Self {
		PlaymatchSignatureGroup::from(value).into()
	}
}

/// A platform in the Playmatch catalogue, embedded in identify and game responses.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlaymatchPlatformV2 {
	/// The ID of the platform.
	pub id: Uuid,

	/// The name of the platform.
	pub name: String,

	/// The id of the company that made the platform, absent when unknown.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub company_id: Option<Uuid>,

	/// When the platform was last updated inside Playmatch.
	pub updated_at: DateTime<Utc>,

	/// When the platform was created inside Playmatch.
	pub created_at: DateTime<Utc>,
}

impl From<PlaymatchPlatform> for PlaymatchPlatformV2 {
	fn from(value: PlaymatchPlatform) -> Self {
		PlaymatchPlatformV2 {
			id: value.id,
			name: value.name,
			company_id: value.company_id,
			updated_at: value.updated_at,
			created_at: value.created_at,
		}
	}
}

/// A company in the Playmatch catalogue, embedded in identify and game responses.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlaymatchCompanyV2 {
	/// The ID of the company.
	pub id: Uuid,

	/// The name of the company.
	pub name: String,

	/// When the company was last updated inside Playmatch.
	pub updated_at: DateTime<Utc>,

	/// When the company was created inside Playmatch.
	pub created_at: DateTime<Utc>,
}

impl From<PlaymatchCompany> for PlaymatchCompanyV2 {
	fn from(value: PlaymatchCompany) -> Self {
		PlaymatchCompanyV2 {
			id: value.id,
			name: value.name,
			updated_at: value.updated_at,
			created_at: value.created_at,
		}
	}
}

/// A game in the Playmatch catalogue, embedded in identify and game responses.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlaymatchGameV2 {
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

	/// Whether this game is still present in the current version of its dat file.
	pub current_in_latest_dat: bool,

	/// The version of the last dat file release this game was seen in, absent when unknown.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub last_seen_dat_version: Option<String>,

	/// The id of the last dat file import this game was seen in, absent when unknown.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub last_seen_dat_file_import_id: Option<Uuid>,

	/// When the game was created inside Playmatch.
	pub created_at: DateTime<Utc>,

	/// When the game was last updated inside Playmatch.
	pub updated_at: DateTime<Utc>,
}

impl From<PlaymatchGame> for PlaymatchGameV2 {
	fn from(value: PlaymatchGame) -> Self {
		PlaymatchGameV2 {
			id: value.id,
			name: value.name,
			description: value.description,
			categories: value.categories,
			clone_of: value.clone_of,
			current_in_latest_dat: value.current_in_latest_dat,
			last_seen_dat_version: value.last_seen_dat_version,
			last_seen_dat_file_import_id: value.last_seen_dat_file_import_id,
			created_at: value.created_at,
			updated_at: value.updated_at,
		}
	}
}

/// A single file (ROM entry) of a game, embedded in identify and game responses.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlaymatchGameFileV2 {
	/// The ID of the game file.
	pub id: Uuid,

	/// The ID of the game this file belongs to.
	pub game_id: Uuid,

	/// The name of the file, including extension.
	pub file_name: String,

	/// The size of the file in bytes, absent when the dat provides none.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub file_size_in_bytes: Option<i64>,

	/// The CRC32 checksum of the file, absent when the dat provides none.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub crc: Option<String>,

	/// The MD5 hash of the file, absent when the dat provides none.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub md5: Option<String>,

	/// The SHA1 hash of the file, absent when the dat provides none.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub sha1: Option<String>,

	/// The SHA256 hash of the file, absent when the dat provides none.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub sha256: Option<String>,

	/// The dat-file status of the file, for example `verified`, absent when the dat provides none.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub status: Option<String>,

	/// The serial number of the ROM, absent when the dat provides none.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub serial: Option<String>,

	/// Whether this hash is still present in the current version of its dat file.
	pub current_in_latest_dat: bool,

	/// The version of the last dat file release this hash was seen in, absent when unknown.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub last_seen_dat_version: Option<String>,

	/// The id of the last dat file import this hash was seen in, absent when unknown.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub last_seen_dat_file_import_id: Option<Uuid>,

	/// When the game file was created inside Playmatch.
	pub created_at: DateTime<Utc>,

	/// When the game file was last updated inside Playmatch.
	pub updated_at: DateTime<Utc>,
}

impl From<PlaymatchGameFile> for PlaymatchGameFileV2 {
	fn from(value: PlaymatchGameFile) -> Self {
		PlaymatchGameFileV2 {
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
			current_in_latest_dat: value.current_in_latest_dat,
			last_seen_dat_version: value.last_seen_dat_version,
			last_seen_dat_file_import_id: value.last_seen_dat_file_import_id,
			created_at: value.created_at,
			updated_at: value.updated_at,
		}
	}
}

/// A single candidate from a fuzzy game-name search. Carries just enough to let
/// a caller pick a result and follow up with a get-game call by id.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameNameSearchResultV2 {
	/// The ID of the game.
	pub id: Uuid,

	/// The name of the game.
	pub name: String,

	/// The ID of the platform this game belongs to.
	pub platform_id: Uuid,

	/// The name of the platform this game belongs to.
	pub platform_name: String,
}

impl From<GameNameSearchResult> for GameNameSearchResultV2 {
	fn from(value: GameNameSearchResult) -> Self {
		GameNameSearchResultV2 {
			id: value.id,
			name: value.name,
			platform_id: value.platform_id,
			platform_name: value.platform_name,
		}
	}
}
