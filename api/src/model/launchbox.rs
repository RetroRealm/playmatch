use serde::{Deserialize, Serialize};
use utoipa::IntoParams;

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct LbIdQuery {
	/// The LaunchBox id of the game.
	pub id: i64,
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct LbGameIdQuery {
	/// The LaunchBox id of the game.
	pub game_id: i64,
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct LbSearchQuery {
	/// The search term.
	pub query: String,
	/// The LaunchBox platform name to narrow the search to, absent to search all platforms.
	#[param(required = false)]
	pub platform_name: Option<String>,
}
