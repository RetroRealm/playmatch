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

/// A User inside Playmatch.
#[derive(Debug, Serialize, Deserialize, Clone, Builder, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct User {
	pub id: Uuid,
	pub discord_id: Option<i64>,
	pub username: String,
	pub api_key: Option<String>,
	pub permissions: UserPermissions,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

/// Permission Levels a user can have in Playmatch.
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
			api_key: value.api_key,
			permissions: value.permissions.into(),
			created_at: value.created_at.into(),
			updated_at: value.updated_at.into(),
		}
	}
}
