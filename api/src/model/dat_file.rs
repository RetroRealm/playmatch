use serde::{Deserialize, Serialize};
use service::entities::dat_file::HashLookup;
use utoipa::IntoParams;
use uuid::Uuid;

/// Filters for the v2 keyset-paginated dat-file catalogue browse. Pagination is
/// supplied separately via the shared `PageParams`.
#[derive(Debug, Clone, Serialize, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct DatFileBrowseQuery {
	/// Restrict to dat files published by this signature group. If omitted, all signature groups are included.
	#[param(required = false)]
	pub signature_group_id: Option<Uuid>,

	/// Restrict to dat files targeting this platform. If omitted, all platforms are included.
	#[param(required = false)]
	pub platform_id: Option<Uuid>,

	/// Restrict to dat files for a platform made by this company. If omitted, all companies are included.
	#[param(required = false)]
	pub company_id: Option<Uuid>,

	/// Restrict to dat files of this subset. If omitted, all subsets are included.
	#[param(required = false)]
	pub subset: Option<String>,

	/// Restrict to dat files carrying this tag. If omitted, all tags are included.
	#[param(required = false)]
	pub tag: Option<String>,

	/// Case-insensitive substring to match against the dat file name. If omitted, names are not filtered.
	#[param(required = false)]
	pub name: Option<String>,
}

/// Granularity of a reverse lookup. `dat` returns one entry per dat file; `group`
/// collapses the result to the publishing signature groups. Defaults to `dat`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ReverseLookupLevel {
	#[default]
	Dat,
	Group,
}

impl ReverseLookupLevel {
	pub fn as_groups(self) -> bool {
		matches!(self, ReverseLookupLevel::Group)
	}
}

/// Query for the v2 hash reverse lookup. At least one hash must be supplied;
/// the strongest one resolves the file (sha256 > sha1 > md5 > crc).
#[derive(Debug, Clone, Default, Serialize, Deserialize, IntoParams)]
pub struct DatFileByHashQuery {
	/// SHA256 hash of the file, 64 hex characters.
	#[param(required = false)]
	pub sha256: Option<String>,

	/// SHA1 hash of the file, 40 hex characters.
	#[param(required = false)]
	pub sha1: Option<String>,

	/// MD5 hash of the file, 32 hex characters.
	#[param(required = false)]
	pub md5: Option<String>,

	/// CRC32 checksum of the file, 8 hex characters.
	#[param(required = false)]
	pub crc: Option<String>,

	/// Granularity of the reverse lookup. `dat` returns one entry per dat file; `group` collapses the result to the publishing signature groups. Defaults to `dat`.
	#[param(required = false)]
	pub level: Option<ReverseLookupLevel>,
}

fn normalize(value: Option<String>) -> Option<String> {
	value.filter(|s| !s.is_empty())
}

fn validate_hex(value: &Option<String>, expected_len: usize, label: &str) -> Result<(), String> {
	if let Some(v) = value.as_deref().filter(|s| !s.is_empty()) {
		if v.len() != expected_len {
			return Err(format!("{label} must be {expected_len} hex characters"));
		}
		if !v.chars().all(|c| c.is_ascii_hexdigit()) {
			return Err(format!("{label} must be hexadecimal"));
		}
	}
	Ok(())
}

impl DatFileByHashQuery {
	pub fn validate(&self) -> Result<(), String> {
		validate_hex(&self.sha256, 64, "sha256")?;
		validate_hex(&self.sha1, 40, "sha1")?;
		validate_hex(&self.md5, 32, "md5")?;
		validate_hex(&self.crc, 8, "crc")?;

		let lookup = self.to_lookup();
		if lookup.is_empty() {
			return Err("at least one of sha256, sha1, md5 or crc is required".to_string());
		}
		Ok(())
	}

	pub fn to_lookup(&self) -> HashLookup {
		HashLookup {
			sha256: normalize(self.sha256.clone()),
			sha1: normalize(self.sha1.clone()),
			md5: normalize(self.md5.clone()),
			crc: normalize(self.crc.clone()),
		}
	}
}

/// Options for the v2 keyset-paginated listing of games within a dat file.
/// Pagination is supplied separately via the shared `PageParams`.
#[derive(Debug, Clone, Serialize, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct DatFileGamesQuery {
	/// Only return games present in the current dat release. Defaults to `true`.
	#[param(required = false)]
	pub current_only: Option<bool>,

	/// Whether to include each game's files in the response. Defaults to `false`.
	#[param(required = false)]
	pub include_files: Option<bool>,

	/// Whether to include each game's external metadata mappings in the response. Defaults to `false`.
	#[param(required = false)]
	pub include_mappings: Option<bool>,
}

/// Options for the v2 keyset-paginated listing of the dat files under a signature
/// group. Pagination is supplied separately via the shared `PageParams`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct SignatureGroupDatFilesQuery {
	/// Restrict to dat files targeting this platform. If omitted, all platforms are included.
	#[param(required = false)]
	pub platform_id: Option<Uuid>,
}

/// Options for the v2 keyset-paginated listing of games across a signature
/// group's dat files. Pagination is supplied separately via the shared
/// `PageParams`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct SignatureGroupGamesQuery {
	/// Restrict to games whose dat file targets this platform. If omitted, all platforms are included.
	#[param(required = false)]
	pub platform_id: Option<Uuid>,

	/// Only return games present in the current dat release. Defaults to `true`.
	#[param(required = false)]
	pub current_only: Option<bool>,
}

/// Options for the game-centric reverse lookup of the dat files a game appears in.
#[derive(Debug, Clone, Default, Serialize, Deserialize, IntoParams)]
pub struct GameDatFilesQuery {
	/// Granularity of the reverse lookup. `dat` returns one entry per dat file; `group` collapses the result to the publishing signature groups. Defaults to `dat`.
	#[param(required = false)]
	pub level: Option<ReverseLookupLevel>,
}

#[cfg(test)]
mod tests {
	use super::*;

	fn query(sha1: Option<&str>, crc: Option<&str>) -> DatFileByHashQuery {
		DatFileByHashQuery {
			sha1: sha1.map(str::to_string),
			crc: crc.map(str::to_string),
			..Default::default()
		}
	}

	#[test]
	fn validate_requires_at_least_one_hash() {
		assert!(query(None, None).validate().is_err());
		assert!(query(Some(""), Some("")).validate().is_err());
	}

	#[test]
	fn validate_rejects_wrong_length_and_non_hex() {
		assert!(query(Some("abc"), None).validate().is_err());
		assert!(query(None, Some("zzzzzzzz")).validate().is_err());
	}

	#[test]
	fn validate_accepts_a_well_formed_hash_and_drops_empties() {
		let q = query(Some("da39a3ee5e6b4b0d3255bfef95601890afd80709"), Some(""));
		assert!(q.validate().is_ok());
		let lookup = q.to_lookup();
		assert!(lookup.sha1.is_some());
		assert!(lookup.crc.is_none());
	}

	#[test]
	fn level_maps_to_group_flag() {
		assert!(!ReverseLookupLevel::default().as_groups());
		assert!(ReverseLookupLevel::Group.as_groups());
	}
}
