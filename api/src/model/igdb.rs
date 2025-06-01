use serde::{Deserialize, Serialize};
use utoipa::IntoParams;

/// Query an entity by its ID
#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct IdQuery {
	pub id: i32,
}

/// Query an entity by its slug or ID
#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct SlugIdQuery {
	pub slug: Option<String>,
	pub id: Option<i32>,
}

/// Query multiple entities by their IDs
#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct IdsQuery {
	pub ids: Vec<i32>,
}

/// Query to search for entities by a string
#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct SearchQuery {
	pub query: String,
}
