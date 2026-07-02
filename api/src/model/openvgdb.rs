use serde::{Deserialize, Serialize};
use utoipa::IntoParams;

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct OvgdbReleaseIdQuery {
	/// The OpenVGDB id of the release.
	pub release_id: i64,
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct OvgdbHashQuery {
	/// The SHA1 hash of the ROM, checked first when present.
	#[param(required = false)]
	pub sha1: Option<String>,
	/// The MD5 hash of the ROM, checked when `sha1` is absent.
	#[param(required = false)]
	pub md5: Option<String>,
	/// The CRC32 hash of the ROM, checked when `sha1` and `md5` are absent.
	#[param(required = false)]
	pub crc: Option<String>,
}
