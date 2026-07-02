use serde::{Deserialize, Serialize};
use service::providers::steamgriddb::model::{AssetFilters, SgdbPlatform, SgdbTriState};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct SgdbIdQuery {
	/// The SteamGridDB id of the game.
	pub id: i64,
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct SgdbPlatformQuery {
	/// The external platform the game id belongs to.
	pub platform: SgdbPlatform,
	/// The game's id on the external platform.
	pub platform_id: String,
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct SgdbSearchQuery {
	/// The search term.
	pub query: String,
}

/// Optional filter knobs forwarded to SGDB on every asset endpoint.
/// Comma-separated string fields accept whatever values SGDB accepts; the
/// proxy does not pre-validate the enum values, so future SGDB additions
/// flow through without a code change.
#[derive(Debug, Default, Serialize, Deserialize, IntoParams, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct SgdbAssetFilterQuery {
	/// Comma-separated list of SGDB style values to filter to.
	pub styles: Option<String>,
	/// Comma-separated list of SGDB dimension values to filter to.
	pub dimensions: Option<String>,
	/// Comma-separated list of SGDB mime type values to filter to.
	pub mimes: Option<String>,
	/// Comma-separated list of SGDB asset type values to filter to.
	pub types: Option<String>,
	/// Whether to include, exclude, or allow either NSFW-tagged assets.
	pub nsfw: Option<SgdbTriState>,
	/// Whether to include, exclude, or allow either humor-tagged assets.
	pub humor: Option<SgdbTriState>,
	/// Whether to include, exclude, or allow either epilepsy-tagged assets.
	pub epilepsy: Option<SgdbTriState>,
	/// Comma-separated list of tags where at least one must match.
	pub oneoftag: Option<String>,
	/// The maximum number of assets to return.
	pub limit: Option<u32>,
	/// The page of results to return.
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
	/// The SteamGridDB id of the game.
	pub game_id: i64,
	#[serde(flatten)]
	pub filters: SgdbAssetFilterQuery,
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct SgdbPlatformAssetQuery {
	/// The external platform the game id belongs to.
	pub platform: SgdbPlatform,
	/// The game's id on the external platform.
	pub platform_id: String,
	#[serde(flatten)]
	pub filters: SgdbAssetFilterQuery,
}
