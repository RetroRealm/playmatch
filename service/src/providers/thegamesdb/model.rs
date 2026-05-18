use serde::{Deserialize, Serialize};

/// Envelope every TheGamesDB v1 response wraps the payload in. We only read the
/// fields we need: the rate-limit metadata and the typed `data` payload.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TgdbEnvelope<T> {
	pub code: i32,
	#[serde(default)]
	pub status: Option<String>,
	#[serde(default)]
	pub remaining_monthly_allowance: Option<i32>,
	#[serde(default)]
	pub extra_allowance: Option<i32>,
	pub data: T,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TgdbSearchData {
	#[serde(default)]
	pub count: i32,
	#[serde(default)]
	pub games: Vec<TgdbApiGame>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TgdbGameByIdData {
	#[serde(default)]
	pub count: i32,
	#[serde(default)]
	pub games: Vec<TgdbApiGame>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TgdbApiGame {
	pub id: i64,
	#[serde(default)]
	pub game_title: Option<String>,
	#[serde(default)]
	pub alternates: Option<Vec<String>>,
}
