use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GameFileRequest {
    pub file_name: String,
    pub file_size: i32,
    pub crc: String,
    pub md5: String,
    pub sha1: String,
    pub sha256: String,
}
