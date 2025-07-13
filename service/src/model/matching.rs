use crate::model::{ManualMatchMode, MetadataProvider};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub trait MatchRequest {
	/// The type of manual match, if your permission level is not Automation this is ignored and set to your permission level instead.
	fn get_manual_match_type(&self) -> ManualMatchMode;

	fn set_manual_match_type(&mut self, match_mode: ManualMatchMode) -> &mut Self;
}

impl MatchRequest for CompanyMatchRequest {
	fn get_manual_match_type(&self) -> ManualMatchMode {
		self.manual_match_type
	}

	fn set_manual_match_type(&mut self, match_mode: ManualMatchMode) -> &mut Self {
		self.manual_match_type = match_mode;
		self
	}
}

impl MatchRequest for PlatformMatchRequest {
	fn get_manual_match_type(&self) -> ManualMatchMode {
		self.manual_match_type
	}

	fn set_manual_match_type(&mut self, match_mode: ManualMatchMode) -> &mut Self {
		self.manual_match_type = match_mode;
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
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompanyMatchRequest {
	/// Name of the Company to match.
	pub name: String,

	/// Optional comment about the match.
	pub comment: Option<String>,

	/// Metadata provider to match for.
	pub provider: MetadataProvider,

	/// ID of the game file in the metadata provider.
	pub provider_id: String,

	/// The type of manual match, if your permission level is not Automation or Admin this is ignored and set to your permission level instead.
	pub manual_match_type: ManualMatchMode,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformMatchRequest {
	/// Name of the Platform to match.
	pub name: String,

	/// Optional comment about the match.
	pub comment: Option<String>,

	/// Metadata provider to match for.
	pub provider: MetadataProvider,

	/// ID of the game file in the metadata provider.
	pub provider_id: String,

	/// The type of manual match, if your permission level is not Automation or Admin this is ignored and set to your permission level instead.
	pub manual_match_type: ManualMatchMode,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameMatchRequest {
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

	/// The type of manual match, if your permission level is not Automation or Admin this is ignored and set to your permission level instead.
	pub manual_match_type: ManualMatchMode,
}
