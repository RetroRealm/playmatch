use entity::{retroachievements_game, retroachievements_game_hash, retroachievements_system};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// RA `API_GetConsoleIDs.php` response entry.
#[derive(Debug, Clone, Deserialize)]
pub struct RaConsole {
	#[serde(rename = "ID", alias = "id")]
	pub id: i32,
	#[serde(rename = "Name", alias = "name")]
	pub name: String,
}

/// RA `API_GetGameList.php` response entry. `Hashes` only present when the
/// request was made with `h=1`.
#[derive(Debug, Clone, Deserialize)]
pub struct RaGameListEntry {
	#[serde(rename = "ID")]
	pub id: i64,
	#[serde(rename = "Title")]
	pub title: String,
	#[serde(rename = "ConsoleID")]
	pub console_id: i32,
	#[serde(rename = "ConsoleName")]
	pub console_name: String,
	#[serde(rename = "ImageIcon", default)]
	pub image_icon: Option<String>,
	#[serde(rename = "NumAchievements", default)]
	pub num_achievements: i32,
	#[serde(rename = "NumLeaderboards", default)]
	pub num_leaderboards: i32,
	#[serde(rename = "Points", default)]
	pub points: i32,
	#[serde(rename = "DateModified", default)]
	pub date_modified: Option<String>,
	#[serde(rename = "ForumTopicID", default)]
	pub forum_topic_id: Option<i64>,
	#[serde(rename = "Hashes", default)]
	pub hashes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RaSystem {
	pub system_id: i32,
	pub name: String,
}

impl From<retroachievements_system::Model> for RaSystem {
	fn from(m: retroachievements_system::Model) -> Self {
		Self {
			system_id: m.system_id,
			name: m.name,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RaGame {
	pub game_id: i64,
	pub system_id: i32,
	pub system_name: String,
	pub title: String,
	pub image_icon: Option<String>,
	pub num_achievements: i32,
	pub num_leaderboards: i32,
	pub points: i32,
	pub date_modified: Option<String>,
	pub forum_topic_id: Option<i64>,
}

impl From<retroachievements_game::Model> for RaGame {
	fn from(m: retroachievements_game::Model) -> Self {
		Self {
			game_id: m.game_id,
			system_id: m.system_id,
			system_name: m.system_name,
			title: m.title,
			image_icon: m.image_icon,
			num_achievements: m.num_achievements,
			num_leaderboards: m.num_leaderboards,
			points: m.points,
			date_modified: m.date_modified,
			forum_topic_id: m.forum_topic_id,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RaGameHash {
	pub game_id: i64,
	pub md5: String,
}

impl From<retroachievements_game_hash::Model> for RaGameHash {
	fn from(m: retroachievements_game_hash::Model) -> Self {
		Self {
			game_id: m.game_id,
			md5: m.md5,
		}
	}
}

/// Combined response: a matched game with all of its hashes.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RaGameMatch {
	pub game: RaGame,
	pub hashes: Vec<RaGameHash>,
}
