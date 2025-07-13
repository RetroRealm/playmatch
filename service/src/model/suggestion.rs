use crate::model::{ManualMatchMode, MetadataProvider};
use sea_orm::sqlx::types::uuid;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameSuggestionRequest {
	/// MD5 hash of the game file.
	pub md5: Option<String>,

	/// SHA1 hash of the game file.
	pub sha1: Option<String>,

	/// SHA256 hash of the game file.
	pub sha256: Option<String>,

	/// Name of game or file.
	pub name: Option<String>,

	/// Optional comment about the match.
	pub comment: Option<String>,

	/// Metadata provider to match for.
	pub provider: MetadataProvider,

	/// ID of the game file in the metadata provider.
	pub provider_id: String,

	/// The type of manual match, if your permission level is not Automation or Admin, this is ignored and set to your permission level instead.
	pub manual_match_type: ManualMatchMode,

	/// The id of the user making the suggestion, if your permission level is not Automation or Admin, this is ignored and set to your user id instead.
	pub user_id: Option<uuid::Uuid>,
}
