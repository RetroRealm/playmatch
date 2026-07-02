use crate::model::MetadataProvider;
use chrono::{DateTime, Utc};
use entity::signature_metadata_mapping_suggestions::Model;
use sea_orm::sqlx::types::uuid;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameSuggestionRequest {
	/// The MD5 hash of the game file.
	pub md5: Option<String>,

	/// The SHA1 hash of the game file.
	pub sha1: Option<String>,

	/// The SHA256 hash of the game file.
	pub sha256: Option<String>,

	/// The name of the game or file.
	pub name: Option<String>,

	/// A comment about the match.
	pub comment: Option<String>,

	/// The metadata provider to match for.
	pub provider: MetadataProvider,

	/// The id of the game in the metadata provider.
	pub provider_id: String,

	/// The id of the user making the suggestion.
	pub user_id: Option<Uuid>,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompanyOrPlatformSuggestionRequest {
	/// The name of the company or platform.
	pub name: String,

	/// A comment about the match.
	pub comment: Option<String>,

	/// The metadata provider to match for.
	pub provider: MetadataProvider,

	/// The id of the company or platform in the metadata provider.
	pub provider_id: String,

	/// The id of the user making the suggestion.
	pub user_id: Option<Uuid>,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
	/// Unique identifier for the suggestion.
	pub id: Uuid,

	/// The id of the game this suggestion is for, absent when the suggestion is not for a game.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub game_id: Option<Uuid>,

	/// The id of the company this suggestion is for, absent when the suggestion is not for a company.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub company_id: Option<Uuid>,

	/// The id of the platform this suggestion is for, absent when the suggestion is not for a platform.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub platform_id: Option<Uuid>,

	/// The provider of the metadata.
	pub provider: MetadataProvider,

	/// The ID of the game, company, or platform on the metadata provider.
	pub provider_id: String,

	/// A comment about the suggestion.
	pub comment: Option<String>,

	/// The id of the user who created the suggestion, absent for externally-submitted ones.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub created_by: Option<Uuid>,

	/// Origin of an externally-submitted suggestion (for example a truncated User-Agent), absent for user-submitted ones.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub source: Option<String>,

	/// When the suggestion was created.
	pub created_at: DateTime<Utc>,

	/// When the suggestion was last updated.
	pub updated_at: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdatedMetadataMatchesFromSuggestionResponse {
	pub updated: i32,
}

impl From<Model> for Suggestion {
	fn from(value: Model) -> Self {
		Self {
			id: value.id,
			game_id: value.game_id,
			company_id: value.company_id,
			platform_id: value.platform_id,
			provider: value.provider.into(),
			provider_id: value.provider_id,
			comment: value.comment,
			created_by: value.created_by,
			source: value.source,
			created_at: value.created_at.into(),
			updated_at: value.updated_at.into(),
		}
	}
}
