use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, IntoParams, Serialize, Deserialize, ToSchema)]
pub struct GetUserQuery {
	pub discord_id: i64,
}
