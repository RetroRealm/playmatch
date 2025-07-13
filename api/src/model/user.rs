use serde::{Deserialize, Serialize};
use service::model::user::UserPermissions;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, IntoParams, Serialize, Deserialize, ToSchema)]
pub struct GetUserQuery {
	pub discord_id: i64,
}

#[derive(Debug, IntoParams, Serialize, Deserialize, ToSchema)]
pub struct UpdateUserPermissionsQuery {
	pub new_permission: UserPermissions,
}
