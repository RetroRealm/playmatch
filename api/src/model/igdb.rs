use serde::{Deserialize, Serialize};
use utoipa::IntoParams;

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct GameIdQuery {
	pub id: i64,
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct GameIdsQuery {
	pub ids: Vec<i64>,
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct GameSearchQuery {
	pub query: String,
}
