use serde::{Deserialize, Serialize};
use utoipa::IntoParams;

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct OvgdbReleaseIdQuery {
	pub release_id: i64,
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct OvgdbHashQuery {
	#[param(required = false)]
	pub sha1: Option<String>,
	#[param(required = false)]
	pub md5: Option<String>,
	#[param(required = false)]
	pub crc: Option<String>,
}
