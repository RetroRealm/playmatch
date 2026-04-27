use serde::{Deserialize, Serialize};
use utoipa::IntoParams;

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct MgIdQuery {
	pub id: i64,
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct MgSearchQuery {
	pub query: String,
	#[param(required = false)]
	pub platform_id: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct MgGamePlatformQuery {
	pub game_id: i64,
	pub platform_id: i64,
}
