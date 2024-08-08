use derive_builder::Builder;
use serde_derive::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct GameFileRequest {
    pub file_name: String,
    pub file_size: i32,
    pub crc: Option<String>,
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
pub struct GameMatchResponse {
    pub game_match_type: GameMatchType,
    pub playmatch_id: Option<Uuid>,
    pub igdb_id: Option<i32>,
    pub mobygames_id: Option<i32>,
}
