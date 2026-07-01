use std::fmt;

/// A path-segment API version. The wire form is the lowercase `vN` string used
/// in `/api/vN` and in the `Playmatch-Api-Version` response header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApiVersion {
	V1,
	V2,
}

impl ApiVersion {
	pub const fn as_str(self) -> &'static str {
		match self {
			ApiVersion::V1 => "v1",
			ApiVersion::V2 => "v2",
		}
	}

	pub fn parse(input: &str) -> Option<Self> {
		match input.trim().to_ascii_lowercase().as_str() {
			"v1" => Some(ApiVersion::V1),
			"v2" => Some(ApiVersion::V2),
			_ => None,
		}
	}
}

impl fmt::Display for ApiVersion {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionStatus {
	Active,
	Deprecated,
	Sunset,
	Removed,
}

impl VersionStatus {
	pub const fn as_str(self) -> &'static str {
		match self {
			VersionStatus::Active => "active",
			VersionStatus::Deprecated => "deprecated",
			VersionStatus::Sunset => "sunset",
			VersionStatus::Removed => "removed",
		}
	}
}

/// Single source of truth for every version's lifecycle. Drives both the
/// always-on version header and any future discovery document. Lifecycle dates
/// are ISO `YYYY-MM-DD` strings parsed lazily so the table stays a plain const.
#[derive(Debug, Clone, Copy)]
pub struct ApiVersionInfo {
	pub version: ApiVersion,
	pub status: VersionStatus,
	pub is_default: bool,
	pub released_on: Option<&'static str>,
	pub deprecated_on: Option<&'static str>,
	pub sunset_on: Option<&'static str>,
	pub removed_on: Option<&'static str>,
	pub docs_url: Option<&'static str>,
	pub openapi_url: Option<&'static str>,
}

/// Compile-time default version. Overridable once at boot via
/// `PLAYMATCH_DEFAULT_API_VERSION`; never resolved per request.
pub const DEFAULT_API_VERSION: ApiVersion = ApiVersion::V1;

pub const API_VERSIONS: &[ApiVersionInfo] = &[
	ApiVersionInfo {
		version: ApiVersion::V1,
		status: VersionStatus::Active,
		is_default: true,
		released_on: Some("2024-01-01"),
		deprecated_on: None,
		sunset_on: None,
		removed_on: None,
		docs_url: Some("/swagger-ui/"),
		openapi_url: Some("/api-docs/v1/openapi.json"),
	},
	ApiVersionInfo {
		version: ApiVersion::V2,
		status: VersionStatus::Active,
		is_default: false,
		released_on: None,
		deprecated_on: None,
		sunset_on: None,
		removed_on: None,
		docs_url: Some("/swagger-ui/"),
		openapi_url: Some("/api-docs/v2/openapi.json"),
	},
];

pub fn version_info(version: ApiVersion) -> Option<&'static ApiVersionInfo> {
	API_VERSIONS.iter().find(|v| v.version == version)
}

/// The newest version a client should target: the last non-removed entry in the
/// registry, which is declared in ascending order.
pub fn latest_version() -> ApiVersion {
	API_VERSIONS
		.iter()
		.rev()
		.find(|v| v.status != VersionStatus::Removed)
		.map(|v| v.version)
		.unwrap_or(DEFAULT_API_VERSION)
}

/// Resolves the default version once at boot. `PLAYMATCH_DEFAULT_API_VERSION`
/// ("v1"|"v2") overrides the compiled `DEFAULT_API_VERSION`; an unset or
/// unparseable value falls back to the compiled default.
pub fn resolve_default_version() -> ApiVersion {
	match std::env::var("PLAYMATCH_DEFAULT_API_VERSION") {
		Ok(raw) => ApiVersion::parse(&raw).unwrap_or(DEFAULT_API_VERSION),
		Err(_) => DEFAULT_API_VERSION,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use chrono::NaiveDate;

	fn parse_date(label: &str, raw: Option<&str>) -> Option<NaiveDate> {
		raw.map(|s| {
			NaiveDate::parse_from_str(s, "%Y-%m-%d")
				.unwrap_or_else(|_| panic!("{label} date `{s}` must be ISO YYYY-MM-DD"))
		})
	}

	#[test]
	fn exactly_one_active_default_exists() {
		let defaults: Vec<_> = API_VERSIONS.iter().filter(|v| v.is_default).collect();
		assert_eq!(
			defaults.len(),
			1,
			"the registry must declare exactly one default version"
		);
		let default = defaults[0];
		assert_eq!(
			default.status,
			VersionStatus::Active,
			"the default version must be active"
		);
		assert_eq!(
			default.version, DEFAULT_API_VERSION,
			"the registry default must match DEFAULT_API_VERSION"
		);
	}

	#[test]
	fn registry_has_no_duplicate_versions() {
		for info in API_VERSIONS {
			let count = API_VERSIONS
				.iter()
				.filter(|v| v.version == info.version)
				.count();
			assert_eq!(count, 1, "version {} appears more than once", info.version);
		}
	}

	#[test]
	fn lifecycle_dates_are_monotone() {
		for info in API_VERSIONS {
			let released = parse_date("released_on", info.released_on);
			let deprecated = parse_date("deprecated_on", info.deprecated_on);
			let sunset = parse_date("sunset_on", info.sunset_on);
			let removed = parse_date("removed_on", info.removed_on);

			let mut stages = Vec::new();
			if let Some(d) = released {
				stages.push(("released", d));
			}
			if let Some(d) = deprecated {
				stages.push(("deprecated", d));
			}
			if let Some(d) = sunset {
				stages.push(("sunset", d));
			}
			if let Some(d) = removed {
				stages.push(("removed", d));
			}

			for pair in stages.windows(2) {
				let (prev_label, prev) = pair[0];
				let (next_label, next) = pair[1];
				assert!(
					prev <= next,
					"version {}: {prev_label} ({prev}) must not be after {next_label} ({next})",
					info.version
				);
			}
		}
	}

	#[test]
	fn status_implies_lifecycle_dates() {
		for info in API_VERSIONS {
			match info.status {
				VersionStatus::Deprecated => assert!(
					info.deprecated_on.is_some(),
					"deprecated version {} must carry deprecated_on",
					info.version
				),
				VersionStatus::Sunset => assert!(
					info.sunset_on.is_some(),
					"sunset version {} must carry sunset_on",
					info.version
				),
				VersionStatus::Removed => assert!(
					info.removed_on.is_some(),
					"removed version {} must carry removed_on",
					info.version
				),
				VersionStatus::Active => {}
			}
		}
	}

	#[test]
	fn parse_is_case_insensitive_and_trims() {
		assert_eq!(ApiVersion::parse(" V1 "), Some(ApiVersion::V1));
		assert_eq!(ApiVersion::parse("v2"), Some(ApiVersion::V2));
		assert_eq!(ApiVersion::parse("v3"), None);
		assert_eq!(ApiVersion::parse(""), None);
	}
}
