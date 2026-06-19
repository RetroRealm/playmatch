use serde::{Deserialize, Serialize};
use utoipa::IntoParams;
use uuid::Uuid;

/// Query to fuzzy-search games by human title.
#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct GameSearchQuery {
	/// The human game title to search for.
	pub query: String,

	/// Optional platform id to narrow the search to a single platform.
	#[param(required = false)]
	pub platform_id: Option<Uuid>,

	/// Optional maximum number of candidates to return.
	#[param(required = false)]
	pub limit: Option<u64>,
}
