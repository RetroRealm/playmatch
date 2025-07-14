use crate::model::{ManualMatchMode, MetadataProvider};
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub trait MatchRequest {
	/// The type of manual match, if your permission level is not Automation this is ignored and set to your permission level instead.
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
	/// Name of the Company or Platform to match.
	pub name: String,

	/// Optional comment about the match.
	pub comment: Option<String>,

	/// Metadata provider to match for.
	pub provider: MetadataProvider,

	/// ID of the Company or Platform file in the metadata provider.
	pub provider_id: String,

	/// The type of manual match, if your permission level is not Automation or Admin this is ignored and set to your permission level instead.
	pub manual_match_type: ManualMatchMode,

	/// The id of the user making the suggestion, if your permission level is not Automation or Admin, this is ignored and set to your user id instead.
	pub user_id: Option<Uuid>,
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

	/// The id of the user making the suggestion, if your permission level is not Automation or Admin, this is ignored and set to your user id instead.
	pub user_id: Option<Uuid>,
}

#[derive(Debug)]
pub struct GameMatchData {
	/// Optional comment about the match.
	pub comment: Option<String>,

	/// Metadata provider to match for.
	pub provider: MetadataProvider,

	/// ID of the game file in the metadata provider.
	pub provider_id: String,

	/// The type of manual match, if your permission level is not Automation or Admin this is ignored and set to your permission level instead.
	pub manual_match_type: ManualMatchMode,

	/// The id of the user making the suggestion, if your permission level is not Automation or Admin, this is ignored and set to your user id instead.
	pub user_id: Option<Uuid>,
}

impl From<GameMatchRequest> for GameMatchData {
	fn from(request: GameMatchRequest) -> Self {
		Self {
			comment: request.comment,
			provider: request.provider,
			provider_id: request.provider_id,
			manual_match_type: request.manual_match_type,
			user_id: request.user_id,
		}
	}
}
