use chrono::{DateTime, Utc};
use derive_builder::Builder;
use entity::sea_orm_active_enums::UserPermissionsEnum;
use entity::user::Model;
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

/// Request to update a user's permissions.
#[derive(Debug, IntoParams, Serialize, Deserialize, ToSchema)]
pub struct UpdateUserPermissionsRequest {
	pub new_permission: UserPermissions,
}

/// Request to get or create a user by their Discord ID.
#[derive(Debug, IntoParams, Serialize, Deserialize, ToSchema)]
pub struct CreateOrGetUserRequest {
	pub discord_id: i64,
	pub username: String,
	pub permissions: UserPermissions,
}

/// Request to update a user's permissions.
#[derive(Debug, Clone, IntoParams, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserPermissionsRequestV2 {
	pub new_permission: UserPermissions,
}

impl From<UpdateUserPermissionsRequestV2> for UpdateUserPermissionsRequest {
	fn from(value: UpdateUserPermissionsRequestV2) -> Self {
		UpdateUserPermissionsRequest {
			new_permission: value.new_permission,
		}
	}
}

/// Request to get or create a user by their Discord ID.
#[derive(Debug, Clone, IntoParams, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateOrGetUserRequestV2 {
	pub discord_id: i64,
	pub username: String,
	pub permissions: UserPermissions,
}

impl From<CreateOrGetUserRequestV2> for CreateOrGetUserRequest {
	fn from(value: CreateOrGetUserRequestV2) -> Self {
		CreateOrGetUserRequest {
			discord_id: value.discord_id,
			username: value.username,
			permissions: value.permissions,
		}
	}
}

/// A user account in Playmatch.
#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct User {
	/// The unique id of the user.
	pub id: Uuid,
	/// The user's Discord account id, null when the account is not linked to Discord.
	pub discord_id: Option<i64>,
	/// The username of the user.
	pub username: String,
	/// The user's permission level.
	pub permissions: UserPermissions,
	/// When the user was created.
	pub created_at: DateTime<Utc>,
	/// When the user was last updated.
	pub updated_at: DateTime<Utc>,
}

/// The permission level of a Playmatch user. Levels are ordered `User` < `Trusted` < `Automation` < `Admin`; a route that requires one level accepts any higher level.
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub enum UserPermissions {
	User,
	Trusted,
	Automation,
	Admin,
}

impl From<UserPermissions> for UserPermissionsEnum {
	fn from(value: UserPermissions) -> Self {
		match value {
			UserPermissions::User => UserPermissionsEnum::User,
			UserPermissions::Trusted => UserPermissionsEnum::Trusted,
			UserPermissions::Automation => UserPermissionsEnum::Automation,
			UserPermissions::Admin => UserPermissionsEnum::Admin,
		}
	}
}

impl From<UserPermissionsEnum> for UserPermissions {
	fn from(value: UserPermissionsEnum) -> Self {
		match value {
			UserPermissionsEnum::User => UserPermissions::User,
			UserPermissionsEnum::Trusted => UserPermissions::Trusted,
			UserPermissionsEnum::Automation => UserPermissions::Automation,
			UserPermissionsEnum::Admin => UserPermissions::Admin,
		}
	}
}

impl From<Model> for User {
	fn from(value: Model) -> Self {
		Self {
			id: value.id,
			discord_id: value.discord_id,
			username: value.username,
			permissions: value.permissions.into(),
			created_at: value.created_at.into(),
			updated_at: value.updated_at.into(),
		}
	}
}
