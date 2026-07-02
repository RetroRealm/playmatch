use serde::{Deserialize, Serialize};
use utoipa::IntoParams;

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct MgIdQuery {
	/// The MobyGames id of the game.
	pub id: i64,
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct MgSearchQuery {
	/// The search term.
	pub query: String,
	/// The MobyGames platform id to narrow the search to, absent to search all platforms.
	#[param(required = false)]
	pub platform_id: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct MgGamePlatformQuery {
	/// The MobyGames id of the game.
	pub game_id: i64,
	/// The MobyGames id of the platform.
	pub platform_id: i64,
}
