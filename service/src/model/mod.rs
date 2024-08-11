use derive_builder::Builder;
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Serialize, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct GameFileMatchSearch {
	pub file_name: String,
	pub file_size: i64,
	pub md5: Option<String>,
	pub sha1: Option<String>,
	pub sha256: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub enum GameMatchType {
	SHA256,
	SHA1,
	MD5,
	FileNameAndSize,
	NoMatch,
}

#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameMatchResult {
	pub game_match_type: GameMatchType,
	pub playmatch_id: Option<Uuid>,
	pub igdb_id: Option<i32>,
	pub mobygames_id: Option<i32>,
}
