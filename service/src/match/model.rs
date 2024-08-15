use serde::{Deserialize, Serialize};
use strum::EnumIter;

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, Serialize, Deserialize)]
pub enum MatchType {
	None,
	Automatic,
	Manual,
	Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, Serialize, Deserialize)]
pub enum ManualMatchType {
	User,
	Admin,
	Community,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, Serialize, Deserialize)]
pub enum FailedMatchReason {
	TooManyMatches,
}
