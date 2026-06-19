use chrono::{DateTime, Utc};
use derive_builder::Builder;
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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

	/// Whether this game is still present in the current version of its dat file.
	pub current_in_latest_dat: bool,

	/// Version string of the last dat file release this game was seen in.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub last_seen_dat_version: Option<String>,

	/// Id of the last dat file import this game was seen in.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub last_seen_dat_file_import_id: Option<Uuid>,

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

	/// Whether this hash is still present in the current version of its dat file.
	pub current_in_latest_dat: bool,

	/// Version string of the last dat file release this hash was seen in.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub last_seen_dat_version: Option<String>,

	/// Id of the last dat file import this hash was seen in.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub last_seen_dat_file_import_id: Option<Uuid>,

	/// When the game file was created inside playmatch.
	pub created_at: DateTime<Utc>,

	/// When the game file was last updated inside playmatch.
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
