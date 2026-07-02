use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, IntoParams, Serialize, Deserialize, ToSchema)]
pub struct GetUserQuery {
	/// The Discord id of the user.
	pub discord_id: i64,
}
