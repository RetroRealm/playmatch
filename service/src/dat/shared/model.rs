use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Datafile {
	pub header: Header,

	pub game: Option<Vec<Game>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Game {
	pub description: Option<String>,

	pub game_id: Option<Vec<String>>,

	pub rom: Vec<RomElement>,

	pub name: String,

	pub id: Option<String>,

	pub cloneofid: Option<String>,

	pub category: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RomElement {
	pub name: String,

	pub size: String,

	pub crc: String,

	pub md5: Option<String>,

	pub sha1: Option<String>,

	pub sha256: Option<String>,

	pub serial: Option<String>,

	pub status: Option<Status>,

	pub mia: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Header {
	pub id: Option<String>,

	pub name: String,

	pub description: String,

	pub version: String,

	pub author: String,

	pub homepage: String,

	pub url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Display)]
pub enum Status {
	#[serde(rename = "baddump")]
	Baddump,

	#[serde(rename = "verified")]
	Verified,
}
