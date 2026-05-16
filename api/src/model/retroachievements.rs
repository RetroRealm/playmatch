use serde::{Deserialize, Serialize};
use utoipa::IntoParams;

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct RaIdQuery {
	pub id: i64,
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct RaHashQuery {
	pub md5: String,
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct RaSearchQuery {
	pub query: String,
	#[param(required = false)]
	pub system_name: Option<String>,
}
