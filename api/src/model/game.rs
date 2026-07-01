use serde::{Deserialize, Serialize};
use service::model::{MetadataMatchType, MetadataProvider};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// Query to fuzzy-search games by human title.
#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct GameSearchQuery {
	/// The game title to search for.
	pub query: String,

	/// Restrict to games on this platform. If omitted, all platforms are included.
	#[param(required = false)]
	pub platform_id: Option<Uuid>,

	/// The maximum number of candidates to return.
	#[param(required = false)]
	pub limit: Option<u64>,
}

/// Filters for the v2 keyset-paginated game browse. Pagination is supplied
/// separately via the shared `PageParams`.
#[derive(Debug, Serialize, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct GameBrowseQuery {
	/// Restrict to games on this platform. If omitted, all platforms are included.
	#[param(required = false)]
	pub platform_id: Option<Uuid>,

	/// Restrict to games belonging to this signature group. If omitted, all signature groups are included.
	#[param(required = false)]
	pub signature_group_id: Option<Uuid>,

	/// Restrict to games whose platform was made by this company. If omitted, all companies are included.
	#[param(required = false)]
	pub company_id: Option<Uuid>,

	/// Only return games marked current. Defaults to `true`.
	#[param(required = false)]
	pub current_only: Option<bool>,

	/// Whether to include clone entries in the results. Defaults to `false`.
	#[param(required = false)]
	pub clones: Option<bool>,
}

/// The search literal for the v2 keyset-paginated fuzzy search. Pagination is
/// supplied separately via the shared `PageParams`.
#[derive(Debug, Serialize, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct GameSearchPageQuery {
	/// The game title to search for.
	pub query: String,

	/// Restrict to games on this platform. If omitted, all platforms are included.
	#[param(required = false)]
	pub platform_id: Option<Uuid>,
}

/// Options for the v2 keyset-paginated listing of a game's ROM file records.
#[derive(Debug, Serialize, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct GameFilesQuery {
	/// Only return files still present in the current dat release. Defaults to `true`.
	#[param(required = false)]
	pub current_only: Option<bool>,
}

/// Optional filters for a game's public provider mappings.
#[derive(Debug, Serialize, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct GameMappingsQuery {
	/// Restrict to mappings for a single metadata provider. If omitted, all providers are included.
	#[param(required = false)]
	pub provider: Option<MetadataProvider>,

	/// Restrict to mappings with this match type. If omitted, all match types are included.
	#[param(required = false)]
	pub match_type: Option<MetadataMatchType>,
}

/// Which slice of the parent and clone graph to return around a game. `children`
/// returns the game's clones; `parent` returns its parent; `siblings` returns the
/// other clones of the same parent; `tree` returns the parent together with all of
/// its clones. Defaults to `tree`.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum CloneDirectionParam {
	Children,
	Parent,
	Siblings,
	#[default]
	Tree,
}

/// Direction selector for the v2 game clone graph.
#[derive(Debug, Serialize, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct GameClonesQuery {
	/// Which slice of the parent and clone graph to return. `children` returns the game's clones; `parent` returns its parent; `siblings` returns the other clones of the same parent; `tree` returns the parent together with all of its clones. Defaults to `tree`.
	#[param(required = false)]
	pub direction: Option<CloneDirectionParam>,
}

/// Exact case-insensitive game lookup by name within a required platform.
#[derive(Debug, Serialize, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct GameByNameQuery {
	/// The exact game title to look up. Case-insensitive.
	pub name: String,

	/// The platform to scope the lookup to.
	pub platform_id: Uuid,
}
