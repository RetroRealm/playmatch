use serde::{Deserialize, Serialize};
use utoipa::IntoParams;

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct RaIdQuery {
	/// The RetroAchievements id of the game.
	pub id: i64,
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct RaHashQuery {
	/// The MD5 hash of the ROM.
	pub md5: String,
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct RaSearchQuery {
	/// The search term.
	pub query: String,
	/// The RetroAchievements system name to narrow the search to, absent to search all systems.
	#[param(required = false)]
	pub system_name: Option<String>,
}
