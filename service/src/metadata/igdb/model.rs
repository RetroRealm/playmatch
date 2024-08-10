use serde::{Deserialize, Serialize};

pub type GameResponses = Vec<GameResponse>;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameResponse {
	pub id: i64,
	#[serde(rename = "age_ratings")]
	pub age_ratings: Vec<i64>,
	#[serde(rename = "aggregated_rating")]
	pub aggregated_rating: f64,
	#[serde(rename = "aggregated_rating_count")]
	pub aggregated_rating_count: i64,
	#[serde(rename = "alternative_names")]
	pub alternative_names: Vec<i64>,
	pub artworks: Vec<i64>,
	pub category: i64,
	pub cover: i64,
	#[serde(rename = "created_at")]
	pub created_at: i64,
	#[serde(rename = "external_games")]
	pub external_games: Vec<i64>,
	#[serde(rename = "first_release_date")]
	pub first_release_date: i64,
	pub franchises: Vec<i64>,
	#[serde(rename = "game_modes")]
	pub game_modes: Vec<i64>,
	pub genres: Vec<i64>,
	#[serde(rename = "involved_companies")]
	pub involved_companies: Vec<i64>,
	pub keywords: Vec<i64>,
	pub name: String,
	pub platforms: Vec<i64>,
	#[serde(rename = "player_perspectives")]
	pub player_perspectives: Vec<i64>,
	pub rating: f64,
	#[serde(rename = "rating_count")]
	pub rating_count: i64,
	#[serde(rename = "release_dates")]
	pub release_dates: Vec<i64>,
	pub screenshots: Vec<i64>,
	#[serde(rename = "similar_games")]
	pub similar_games: Vec<i64>,
	pub slug: String,
	pub storyline: String,
	pub summary: String,
	pub tags: Vec<i64>,
	pub themes: Vec<i64>,
	#[serde(rename = "total_rating")]
	pub total_rating: f64,
	#[serde(rename = "total_rating_count")]
	pub total_rating_count: i64,
	#[serde(rename = "updated_at")]
	pub updated_at: i64,
	pub url: String,
	pub videos: Vec<i64>,
	pub websites: Vec<i64>,
	pub checksum: String,
	pub remakes: Vec<i64>,
	#[serde(rename = "expanded_games")]
	pub expanded_games: Vec<i64>,
	#[serde(rename = "language_supports")]
	pub language_supports: Vec<i64>,
	#[serde(rename = "game_localizations")]
	pub game_localizations: Vec<i64>,
	pub collections: Vec<i64>,
}
