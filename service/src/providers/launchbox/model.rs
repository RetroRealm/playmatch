use entity::{
	launchbox_game, launchbox_game_alternate_name, launchbox_game_image, launchbox_platform,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LbPlatform {
	pub name: String,
	pub emulated: Option<bool>,
	pub release_date: Option<String>,
	pub developer: Option<String>,
	pub manufacturer: Option<String>,
	pub cpu: Option<String>,
	pub memory: Option<String>,
	pub graphics: Option<String>,
	pub sound: Option<String>,
	pub display: Option<String>,
	pub media: Option<String>,
	pub max_controllers: Option<String>,
	pub notes: Option<String>,
	pub category: Option<String>,
}

impl From<launchbox_platform::Model> for LbPlatform {
	fn from(m: launchbox_platform::Model) -> Self {
		Self {
			name: m.name,
			emulated: m.emulated,
			release_date: m.release_date,
			developer: m.developer,
			manufacturer: m.manufacturer,
			cpu: m.cpu,
			memory: m.memory,
			graphics: m.graphics,
			sound: m.sound,
			display: m.display,
			media: m.media,
			max_controllers: m.max_controllers,
			notes: m.notes,
			category: m.category,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LbGame {
	pub database_id: i64,
	pub name: String,
	pub platform_name: String,
	pub release_date: Option<String>,
	pub release_year: Option<i32>,
	pub overview: Option<String>,
	pub developer: Option<String>,
	pub publisher: Option<String>,
	pub genres: Option<String>,
	pub max_players: Option<i32>,
	pub cooperative: Option<bool>,
	pub esrb: Option<String>,
	pub release_type: Option<String>,
	pub status: Option<String>,
	pub wikipedia_url: Option<String>,
	pub video_url: Option<String>,
	pub community_rating: Option<f32>,
	pub community_rating_count: Option<i32>,
}

impl From<launchbox_game::Model> for LbGame {
	fn from(m: launchbox_game::Model) -> Self {
		Self {
			database_id: m.database_id,
			name: m.name,
			platform_name: m.platform_name,
			release_date: m.release_date,
			release_year: m.release_year,
			overview: m.overview,
			developer: m.developer,
			publisher: m.publisher,
			genres: m.genres,
			max_players: m.max_players,
			cooperative: m.cooperative,
			esrb: m.esrb,
			release_type: m.release_type,
			status: m.status,
			wikipedia_url: m.wikipedia_url,
			video_url: m.video_url,
			community_rating: m.community_rating,
			community_rating_count: m.community_rating_count,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LbGameAlternateName {
	pub launchbox_game_database_id: i64,
	pub name: String,
	pub region: Option<String>,
}

impl From<launchbox_game_alternate_name::Model> for LbGameAlternateName {
	fn from(m: launchbox_game_alternate_name::Model) -> Self {
		Self {
			launchbox_game_database_id: m.launchbox_game_database_id,
			name: m.name,
			region: m.region,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LbGameImage {
	pub launchbox_game_database_id: i64,
	pub file_name: String,
	pub image_type: String,
	pub region: Option<String>,
}

impl From<launchbox_game_image::Model> for LbGameImage {
	fn from(m: launchbox_game_image::Model) -> Self {
		Self {
			launchbox_game_database_id: m.launchbox_game_database_id,
			file_name: m.file_name,
			image_type: m.image_type,
			region: m.region,
		}
	}
}
