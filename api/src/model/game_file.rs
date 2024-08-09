use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use service::model::{GameFileMatchSearch, GameMatchResult};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct GameFileRequest {
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
pub struct GameMatchResponse {
    pub game_match_type: GameMatchType,
    pub playmatch_id: Option<Uuid>,
    pub igdb_id: Option<i32>,
    pub mobygames_id: Option<i32>,
}

impl From<GameFileRequest> for GameFileMatchSearch {
    fn from(value: GameFileRequest) -> Self {
        GameFileMatchSearch {
            file_name: value.file_name,
            file_size: value.file_size,
            md5: value.md5,
            sha1: value.sha1,
            sha256: value.sha256,
        }
    }
}

impl From<service::model::GameMatchType> for GameMatchType {
    fn from(value: service::model::GameMatchType) -> Self {
        match value {
            service::model::GameMatchType::SHA256 => GameMatchType::SHA256,
            service::model::GameMatchType::SHA1 => GameMatchType::SHA1,
            service::model::GameMatchType::MD5 => GameMatchType::MD5,
            service::model::GameMatchType::FileNameAndSize => GameMatchType::FileNameAndSize,
            service::model::GameMatchType::NoMatch => GameMatchType::NoMatch,
        }
    }
}

impl From<GameMatchResult> for GameMatchResponse {
    fn from(value: GameMatchResult) -> Self {
        GameMatchResponse {
            game_match_type: value.game_match_type.into(),
            playmatch_id: value.playmatch_id,
            igdb_id: value.igdb_id,
            mobygames_id: value.mobygames_id,
        }
    }
}
