use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MgPlatform {
	pub platform_id: i64,
	pub platform_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MgPlatformsResp {
	pub platforms: Vec<MgPlatform>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MgGenre {
	pub genre_id: i64,
	pub genre_name: String,
	pub genre_category: String,
	pub genre_category_id: i64,
	#[serde(default)]
	pub genre_description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MgGenresResp {
	pub genres: Vec<MgGenre>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MgAltTitle {
	pub title: String,
	#[serde(default)]
	pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MgGamePlatformBrief {
	pub platform_id: i64,
	pub platform_name: String,
	#[serde(default)]
	pub first_release_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MgSampleCover {
	pub image: String,
	pub thumbnail_image: String,
	#[serde(default)]
	pub platforms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MgSampleScreenshot {
	pub image: String,
	pub thumbnail_image: String,
	#[serde(default)]
	pub caption: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MgGame {
	pub game_id: i64,
	pub title: String,
	#[serde(default)]
	pub moby_url: Option<String>,
	#[serde(default)]
	pub alternate_titles: Option<Vec<MgAltTitle>>,
	#[serde(default)]
	pub description: Option<String>,
	#[serde(default)]
	pub genres: Option<Vec<MgGenre>>,
	#[serde(default)]
	pub platforms: Option<Vec<MgGamePlatformBrief>>,
	#[serde(default)]
	pub sample_cover: Option<MgSampleCover>,
	#[serde(default)]
	pub sample_screenshots: Option<Vec<MgSampleScreenshot>>,
	#[serde(default)]
	pub moby_score: Option<f32>,
	#[serde(default)]
	pub moby_score_count: Option<i64>,
	#[serde(default)]
	pub official_url: Option<String>,
}

impl MgGame {
	pub fn iter_candidate_titles(&self) -> impl Iterator<Item = &str> {
		std::iter::once(self.title.as_str()).chain(
			self.alternate_titles
				.as_deref()
				.unwrap_or_default()
				.iter()
				.map(|alt| alt.title.as_str()),
		)
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MgGamesResp {
	pub games: Vec<MgGame>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MgCover {
	pub image: String,
	pub thumbnail_image: String,
	#[serde(default)]
	pub height: Option<i64>,
	#[serde(default)]
	pub width: Option<i64>,
	#[serde(default)]
	pub scan_of: Option<String>,
	#[serde(default)]
	pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MgCoverGroup {
	#[serde(default)]
	pub comments: Option<String>,
	#[serde(default)]
	pub countries: Vec<String>,
	pub covers: Vec<MgCover>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MgCoversResp {
	pub cover_groups: Vec<MgCoverGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MgScreenshot {
	pub image: String,
	pub thumbnail_image: String,
	#[serde(default)]
	pub caption: Option<String>,
	#[serde(default)]
	pub height: Option<i64>,
	#[serde(default)]
	pub width: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MgScreenshotsResp {
	pub screenshots: Vec<MgScreenshot>,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn iter_candidate_titles_yields_main_first() {
		let game = MgGame {
			game_id: 1,
			title: "Sonic the Hedgehog".to_string(),
			moby_url: None,
			alternate_titles: Some(vec![
				MgAltTitle {
					title: "ソニック・ザ・ヘッジホッグ".to_string(),
					description: Some("Japanese title".to_string()),
				},
				MgAltTitle {
					title: "Sonic 1".to_string(),
					description: None,
				},
			]),
			description: None,
			genres: None,
			platforms: None,
			sample_cover: None,
			sample_screenshots: None,
			moby_score: None,
			moby_score_count: None,
			official_url: None,
		};
		let titles: Vec<&str> = game.iter_candidate_titles().collect();
		assert_eq!(
			titles,
			vec![
				"Sonic the Hedgehog",
				"ソニック・ザ・ヘッジホッグ",
				"Sonic 1",
			]
		);
	}

	#[test]
	fn iter_candidate_titles_handles_no_alternates() {
		let game = MgGame {
			game_id: 1,
			title: "Tetris".to_string(),
			moby_url: None,
			alternate_titles: None,
			description: None,
			genres: None,
			platforms: None,
			sample_cover: None,
			sample_screenshots: None,
			moby_score: None,
			moby_score_count: None,
			official_url: None,
		};
		let titles: Vec<&str> = game.iter_candidate_titles().collect();
		assert_eq!(titles, vec!["Tetris"]);
	}
}
