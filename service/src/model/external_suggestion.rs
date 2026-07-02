use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Fire-and-forget payload for `POST /api/suggestion/external/game`. The client
/// is identified by User-Agent server-side, not in the body. `mappings` carries
/// every resolved provider id for one ROM in a single request.
#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExternalGameMatchSuggestionPayload {
	/// The MD5 hash of the game file.
	pub md5: Option<String>,

	/// The SHA1 hash of the game file.
	pub sha1: Option<String>,

	/// The SHA256 hash of the game file.
	pub sha256: Option<String>,

	/// The file name of the ROM as the client knows it.
	pub file_name: Option<String>,

	/// The file size in bytes.
	pub file_size: Option<i64>,

	/// Every (provider, providerId) binding the client has resolved for this ROM.
	pub mappings: Vec<ExternalProviderMapping>,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExternalProviderMapping {
	/// The metadata provider tag. Unknown values are accepted and dropped by the
	/// drain worker; only `IGDB` is recognized at the moment.
	pub provider: String,

	/// The provider-side id the client matched the ROM to.
	pub provider_id: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub(crate) struct QueuedExternalSuggestion {
	pub payload: ExternalGameMatchSuggestionPayload,
	pub user_agent: Option<String>,
	pub enqueued_at: DateTime<Utc>,
}
