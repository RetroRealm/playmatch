use serde::{Deserialize, Serialize};
use utoipa::IntoParams;

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct SsIdQuery {
	pub id: i64,
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct SsRomQuery {
	pub system_id: i32,
	pub rom_name: String,
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct SsSearchQuery {
	pub system_id: i32,
	pub query: String,
}
