use entity::{openvgdb_release, openvgdb_rom};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OvgdbRom {
	pub rom_id: i64,
	pub system_id: Option<i32>,
	pub region_id: Option<i32>,
	pub rom_hash_crc: Option<String>,
	pub rom_hash_md5: Option<String>,
	pub rom_hash_sha1: Option<String>,
	pub rom_size: Option<i64>,
	pub rom_file_name: Option<String>,
	pub rom_extensionless_file_name: Option<String>,
	pub rom_serial: Option<String>,
}

impl From<openvgdb_rom::Model> for OvgdbRom {
	fn from(m: openvgdb_rom::Model) -> Self {
		Self {
			rom_id: m.rom_id,
			system_id: m.system_id,
			region_id: m.region_id,
			rom_hash_crc: m.rom_hash_crc,
			rom_hash_md5: m.rom_hash_md5,
			rom_hash_sha1: m.rom_hash_sha1,
			rom_size: m.rom_size,
			rom_file_name: m.rom_file_name,
			rom_extensionless_file_name: m.rom_extensionless_file_name,
			rom_serial: m.rom_serial,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OvgdbRelease {
	pub release_id: i64,
	pub rom_id: i64,
	pub title_name: String,
	pub region_name: Option<String>,
	pub system_name: Option<String>,
	pub cover_front: Option<String>,
	pub cover_back: Option<String>,
	pub description: Option<String>,
	pub developer: Option<String>,
	pub publisher: Option<String>,
	pub genre: Option<String>,
	pub release_date: Option<String>,
	pub release_year: Option<i32>,
	pub reference_url: Option<String>,
}

impl From<openvgdb_release::Model> for OvgdbRelease {
	fn from(m: openvgdb_release::Model) -> Self {
		Self {
			release_id: m.release_id,
			rom_id: m.rom_id,
			title_name: m.title_name,
			region_name: m.region_name,
			system_name: m.system_name,
			cover_front: m.cover_front,
			cover_back: m.cover_back,
			description: m.description,
			developer: m.developer,
			publisher: m.publisher,
			genre: m.genre,
			release_date: m.release_date,
			release_year: m.release_year,
			reference_url: m.reference_url,
		}
	}
}

/// A rom matched by hash together with every release attached to it.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OvgdbRomMatch {
	pub rom: OvgdbRom,
	pub releases: Vec<OvgdbRelease>,
}
