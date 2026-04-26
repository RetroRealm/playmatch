use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SgdbPlatform {
	Steam,
	Origin,
	Egs,
	Bnet,
	Uplay,
	Flashpoint,
	Eshop,
}

impl SgdbPlatform {
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Steam => "steam",
			Self::Origin => "origin",
			Self::Egs => "egs",
			Self::Bnet => "bnet",
			Self::Uplay => "uplay",
			Self::Flashpoint => "flashpoint",
			Self::Eshop => "eshop",
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SgdbGridStyle {
	Alternate,
	Blurred,
	WhiteLogo,
	Material,
	NoLogo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SgdbHeroStyle {
	Alternate,
	Blurred,
	Material,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SgdbLogoStyle {
	Official,
	White,
	Black,
	Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SgdbIconStyle {
	Official,
	Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum SgdbGridDimension {
	#[serde(rename = "460x215")]
	W460H215,
	#[serde(rename = "920x430")]
	W920H430,
	#[serde(rename = "600x900")]
	W600H900,
	#[serde(rename = "342x482")]
	W342H482,
	#[serde(rename = "660x930")]
	W660H930,
	#[serde(rename = "512x512")]
	W512H512,
	#[serde(rename = "1024x1024")]
	W1024H1024,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum SgdbHeroDimension {
	#[serde(rename = "1920x620")]
	W1920H620,
	#[serde(rename = "3840x1240")]
	W3840H1240,
	#[serde(rename = "1600x650")]
	W1600H650,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SgdbAssetMime {
	#[serde(rename = "image/png")]
	Png,
	#[serde(rename = "image/jpeg")]
	Jpeg,
	#[serde(rename = "image/webp")]
	Webp,
	#[serde(rename = "image/vnd.microsoft.icon")]
	Ico,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SgdbAssetType {
	Static,
	Animated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SgdbContentTag {
	Humor,
	Nsfw,
	Epilepsy,
}

/// Tri-state filter SGDB accepts on `nsfw`, `humor`, and `epilepsy` query
/// params. Encoded as `"true"`, `"false"`, or `"any"` on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SgdbTriState {
	True,
	False,
	Any,
}

impl SgdbTriState {
	pub fn as_str(self) -> &'static str {
		match self {
			Self::True => "true",
			Self::False => "false",
			Self::Any => "any",
		}
	}
}

/// Game record returned by `/games/id/{id}`, `/games/{platform}/{id}`, and the
/// search autocomplete endpoint. Field set is small but stable.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SgdbGame {
	pub id: i64,
	pub name: String,
	#[serde(default)]
	pub types: Vec<String>,
	#[serde(default)]
	pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SgdbAuthor {
	pub name: String,
	pub steam64: String,
	pub avatar: String,
}

/// Normalised asset record used for grids, heroes, logos, and icons. The four
/// SGDB asset endpoints return the same shape with slightly different valid
/// styles/dimensions/mimes; we keep them as one struct and let the response
/// stream them through unchanged.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SgdbAsset {
	pub id: i64,
	pub score: i64,
	pub style: String,
	pub width: Option<i64>,
	pub height: Option<i64>,
	pub nsfw: Option<bool>,
	pub humor: Option<bool>,
	pub epilepsy: Option<bool>,
	pub upvotes: Option<i64>,
	pub downvotes: Option<i64>,
	pub url: String,
	pub thumb: String,
	pub mime: Option<String>,
	pub language: Option<String>,
	#[serde(default)]
	pub tags: Vec<String>,
	pub author: SgdbAuthor,
}

/// Filter knobs passed straight through to SGDB's asset endpoints. All fields
/// optional; serialised values are stable so the cache helper can hash them.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct AssetFilters {
	#[serde(default)]
	pub styles: Vec<String>,
	#[serde(default)]
	pub dimensions: Vec<String>,
	#[serde(default)]
	pub mimes: Vec<String>,
	#[serde(default)]
	pub types: Vec<String>,
	pub nsfw: Option<SgdbTriState>,
	pub humor: Option<SgdbTriState>,
	pub epilepsy: Option<SgdbTriState>,
	#[serde(default)]
	pub oneoftag: Vec<String>,
	pub limit: Option<u32>,
	pub page: Option<u32>,
}

impl AssetFilters {
	/// Build a deterministic `&[(name, value)]` list suitable for `reqwest`
	/// query strings and for the cache-key hasher. Sorted by name; empty
	/// vectors omit the param entirely.
	pub fn to_query_pairs(&self) -> Vec<(&'static str, String)> {
		let mut out: Vec<(&'static str, String)> = Vec::new();
		if !self.styles.is_empty() {
			out.push(("styles", self.styles.join(",")));
		}
		if !self.dimensions.is_empty() {
			out.push(("dimensions", self.dimensions.join(",")));
		}
		if !self.mimes.is_empty() {
			out.push(("mimes", self.mimes.join(",")));
		}
		if !self.types.is_empty() {
			out.push(("types", self.types.join(",")));
		}
		if let Some(v) = self.nsfw {
			out.push(("nsfw", v.as_str().to_string()));
		}
		if let Some(v) = self.humor {
			out.push(("humor", v.as_str().to_string()));
		}
		if let Some(v) = self.epilepsy {
			out.push(("epilepsy", v.as_str().to_string()));
		}
		if !self.oneoftag.is_empty() {
			out.push(("oneoftag", self.oneoftag.join(",")));
		}
		if let Some(v) = self.limit {
			out.push(("limit", v.to_string()));
		}
		if let Some(v) = self.page {
			out.push(("page", v.to_string()));
		}
		out.sort_by(|a, b| a.0.cmp(b.0));
		out
	}

	/// Stable, lowercased canonical form for cache-key hashing.
	pub fn canonical_string(&self) -> String {
		self.to_query_pairs()
			.into_iter()
			.map(|(k, v)| format!("{k}={}", v.to_lowercase()))
			.collect::<Vec<_>>()
			.join("&")
	}
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SgdbSingleEnvelope<T> {
	pub success: bool,
	pub data: Option<T>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SgdbListEnvelope<T> {
	pub success: bool,
	pub data: Vec<T>,
}
