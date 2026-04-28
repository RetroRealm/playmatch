use serde::{Deserialize, Serialize};
use utoipa::IntoParams;

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct LbIdQuery {
	pub id: i64,
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct LbGameIdQuery {
	pub game_id: i64,
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct LbSearchQuery {
	pub query: String,
	#[param(required = false)]
	pub platform_name: Option<String>,
}
