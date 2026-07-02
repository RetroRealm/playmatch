use crate::model::{MAX_NAME_INPUT_LEN, ManualMatchMode, MetadataProvider, validate_optional_hex};
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub trait MatchRequest {
	/// Returns the requested manual match type. The match handlers overwrite it
	/// with `Trusted` when the caller holds Trusted permissions; Automation and
	/// Admin callers keep the value they sent.
	fn get_manual_match_type(&self) -> ManualMatchMode;

	fn set_manual_match_type(&mut self, match_mode: ManualMatchMode) -> &mut Self;

	fn get_user_id(&self) -> Option<Uuid>;

	fn set_user_id(&mut self, user_id: Option<Uuid>) -> &mut Self;
}

impl MatchRequest for CompanyOrPlatformMatchRequest {
	fn get_manual_match_type(&self) -> ManualMatchMode {
		self.manual_match_type
	}

	fn set_manual_match_type(&mut self, match_mode: ManualMatchMode) -> &mut Self {
		self.manual_match_type = match_mode;
		self
	}

	fn get_user_id(&self) -> Option<Uuid> {
		self.user_id
	}

	fn set_user_id(&mut self, user_id: Option<Uuid>) -> &mut Self {
		self.user_id = user_id;
		self
	}
}

impl MatchRequest for GameMatchRequest {
	fn get_manual_match_type(&self) -> ManualMatchMode {
		self.manual_match_type
	}

	fn set_manual_match_type(&mut self, match_mode: ManualMatchMode) -> &mut Self {
		self.manual_match_type = match_mode;
		self
	}

	fn get_user_id(&self) -> Option<Uuid> {
		self.user_id
	}

	fn set_user_id(&mut self, user_id: Option<Uuid>) -> &mut Self {
		self.user_id = user_id;
		self
	}
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompanyOrPlatformMatchRequest {
	/// The name of the company or platform to match.
	pub name: String,

	/// A comment about the match.
	pub comment: Option<String>,

	/// The metadata provider to match for.
	pub provider: MetadataProvider,

	/// The id of the company or platform in the metadata provider.
	pub provider_id: String,

	/// The type of manual match. Ignored unless the caller holds Automation or Admin permissions; otherwise set from the caller's permission level.
	pub manual_match_type: ManualMatchMode,

	/// The id of the user making the suggestion. Ignored unless the caller holds Automation or Admin permissions; otherwise set to the caller's user id.
	pub user_id: Option<Uuid>,

	/// The canonical title from the provider, used for cross-provider name propagation.
	pub matched_name: Option<String>,
}

impl CompanyOrPlatformMatchRequest {
	pub fn validate(&self) -> Result<(), String> {
		if self.name.chars().count() > MAX_NAME_INPUT_LEN {
			return Err(format!("name exceeds {MAX_NAME_INPUT_LEN} characters"));
		}
		if self.provider_id.chars().count() > MAX_NAME_INPUT_LEN {
			return Err(format!(
				"providerId exceeds {MAX_NAME_INPUT_LEN} characters"
			));
		}
		Ok(())
	}
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameMatchRequest {
	/// The MD5 hash of the game file, 32 hex characters.
	pub md5: Option<String>,

	/// The SHA1 hash of the game file, 40 hex characters.
	pub sha1: Option<String>,

	/// The SHA256 hash of the game file, 64 hex characters.
	pub sha256: Option<String>,

	/// The name of the game or file. At most 512 characters.
	pub name: Option<String>,

	/// A comment about the match.
	pub comment: Option<String>,

	/// The metadata provider to match for.
	pub provider: MetadataProvider,

	/// The id of the game in the metadata provider.
	pub provider_id: String,

	/// The type of manual match. Ignored unless the caller holds Automation or Admin permissions; otherwise set from the caller's permission level.
	pub manual_match_type: ManualMatchMode,

	/// The id of the user making the suggestion. Ignored unless the caller holds Automation or Admin permissions; otherwise set to the caller's user id.
	pub user_id: Option<Uuid>,

	/// The canonical title from the provider, used for cross-provider name propagation.
	pub matched_name: Option<String>,
}

impl GameMatchRequest {
	pub fn validate(&self) -> Result<(), String> {
		if let Some(name) = &self.name
			&& name.chars().count() > MAX_NAME_INPUT_LEN
		{
			return Err(format!("name exceeds {MAX_NAME_INPUT_LEN} characters"));
		}
		if self.provider_id.chars().count() > MAX_NAME_INPUT_LEN {
			return Err(format!(
				"providerId exceeds {MAX_NAME_INPUT_LEN} characters"
			));
		}
		validate_optional_hex(&self.md5, 32, "md5")?;
		validate_optional_hex(&self.sha1, 40, "sha1")?;
		validate_optional_hex(&self.sha256, 64, "sha256")?;
		Ok(())
	}
}

#[derive(Debug)]
pub struct GameMatchData {
	/// A comment about the match.
	pub comment: Option<String>,

	/// The metadata provider to match for.
	pub provider: MetadataProvider,

	/// The id of the game in the metadata provider.
	pub provider_id: String,

	/// The type of manual match. Ignored unless the caller holds Automation or Admin permissions; otherwise set from the caller's permission level.
	pub manual_match_type: ManualMatchMode,

	/// The id of the user making the suggestion. Ignored unless the caller holds Automation or Admin permissions; otherwise set to the caller's user id.
	pub user_id: Option<Uuid>,

	pub matched_name: Option<String>,
}

impl From<GameMatchRequest> for GameMatchData {
	fn from(request: GameMatchRequest) -> Self {
		Self {
			comment: request.comment,
			provider: request.provider,
			provider_id: request.provider_id,
			manual_match_type: request.manual_match_type,
			user_id: request.user_id,
			matched_name: request.matched_name,
		}
	}
}
