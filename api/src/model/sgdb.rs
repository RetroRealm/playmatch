use serde::{Deserialize, Serialize};
use service::providers::steamgriddb::model::{AssetFilters, SgdbPlatform, SgdbTriState};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct SgdbIdQuery {
	pub id: i64,
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct SgdbPlatformQuery {
	pub platform: SgdbPlatform,
	pub platform_id: String,
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct SgdbSearchQuery {
	pub query: String,
}

/// Optional filter knobs forwarded to SGDB on every asset endpoint.
/// Comma-separated string fields accept whatever values SGDB accepts; the
/// proxy does not pre-validate the enum values, so future SGDB additions
/// flow through without a code change.
#[derive(Debug, Default, Serialize, Deserialize, IntoParams, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct SgdbAssetFilterQuery {
	pub styles: Option<String>,
	pub dimensions: Option<String>,
	pub mimes: Option<String>,
	pub types: Option<String>,
	pub nsfw: Option<SgdbTriState>,
	pub humor: Option<SgdbTriState>,
	pub epilepsy: Option<SgdbTriState>,
	pub oneoftag: Option<String>,
	pub limit: Option<u32>,
	pub page: Option<u32>,
}

impl SgdbAssetFilterQuery {
	pub fn into_filters(self) -> AssetFilters {
		fn split(s: Option<String>) -> Vec<String> {
			s.map(|v| {
				v.split(',')
					.map(str::trim)
					.filter(|p| !p.is_empty())
					.map(str::to_string)
					.collect()
			})
			.unwrap_or_default()
		}
		AssetFilters {
			styles: split(self.styles),
			dimensions: split(self.dimensions),
			mimes: split(self.mimes),
			types: split(self.types),
			nsfw: self.nsfw,
			humor: self.humor,
			epilepsy: self.epilepsy,
			oneoftag: split(self.oneoftag),
			limit: self.limit,
			page: self.page,
		}
	}
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct SgdbGameAssetQuery {
	pub game_id: i64,
	#[serde(flatten)]
	pub filters: SgdbAssetFilterQuery,
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct SgdbPlatformAssetQuery {
	pub platform: SgdbPlatform,
	pub platform_id: String,
	#[serde(flatten)]
	pub filters: SgdbAssetFilterQuery,
}
