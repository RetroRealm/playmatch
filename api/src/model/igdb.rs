use serde::{Deserialize, Serialize};
use utoipa::IntoParams;

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct IdQuery {
	pub id: i32,
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct IdsQuery {
	pub ids: Vec<i32>,
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct SearchQuery {
	pub query: String,
}
