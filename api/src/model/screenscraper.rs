use serde::{Deserialize, Serialize};
use utoipa::IntoParams;

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct SsIdQuery {
	/// The ScreenScraper id of the game.
	pub id: i64,
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct SsRomQuery {
	/// The ScreenScraper id of the system.
	pub system_id: i32,
	/// The exact rom file name to match within the system.
	pub rom_name: String,
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct SsSearchQuery {
	/// The ScreenScraper id of the system to search within.
	pub system_id: i32,
	/// The search term.
	pub query: String,
}
