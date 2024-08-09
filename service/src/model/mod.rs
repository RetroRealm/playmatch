use derive_builder::Builder;
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameFileMatchSearch {
	pub file_name: String,
	pub file_size: i64,
	pub md5: Option<String>,
	pub sha1: Option<String>,
	pub sha256: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum GameMatchType {
	SHA256,
	SHA1,
	MD5,
	FileNameAndSize,
	NoMatch,
}

#[derive(Debug, Serialize, Deserialize, Clone, Builder)]
#[serde(rename_all = "camelCase")]
pub struct GameMatchResult {
	pub game_match_type: GameMatchType,
	pub playmatch_id: Option<Uuid>,
	pub igdb_id: Option<i32>,
	pub mobygames_id: Option<i32>,
}
