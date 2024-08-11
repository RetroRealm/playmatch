use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use utoipa::ToSchema;

pub type GameResponses = Vec<Game>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
pub enum GameCategory {
	MainGame,
	DLCAddon,
	Expansion,
	Bundle,
	StandaloneExpansion,
	Mod,
	Episode,
	Season,
	Remake,
	Remaster,
	ExpandedGame,
	Port,
	Fork,
	Pack,
	Update,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
pub enum GameStatus {
	Released = 0,
	Alpha = 2,
	Beta,
	EarlyAccess,
	Offline,
	Cancelled,
	Rumored,
	Delisted,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Game {
	id: i32,
	category: GameCategory,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	#[serde(skip_serializing_if = "Option::is_none")]
	external_games: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	expanded_games: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	expansions: Option<Vec<i32>>,
	name: String,
	slug: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
	url: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	websites: Option<Vec<i32>>,
	checksum: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	cover: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	game_modes: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	genres: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	involved_companies: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	summary: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	language_supports: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	bundles: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	first_release_date: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	forks: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	platforms: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	player_perspectives: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	release_dates: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	similar_games: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	tags: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	themes: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	artworks: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	screenshots: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	age_ratings: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	aggregated_rating: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	aggregated_rating_count: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	total_rating: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	total_rating_count: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	status: Option<GameStatus>,
	#[serde(skip_serializing_if = "Option::is_none")]
	videos: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	parent_game: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	game_engines: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	keywords: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	storyline: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	rating: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	rating_count: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	alternative_names: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	collection: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	collections: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	game_localizations: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	version_parent: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	version_title: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	follows: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	franchises: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	dlcs: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	hypes: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	multiplayer_modes: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	ports: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	remakes: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	remasters: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	standalone_expansions: Option<Vec<i32>>,
}
