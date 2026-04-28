use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct EmuReadyEnvelope<T> {
	pub result: EmuReadyResult<T>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmuReadyResult<T> {
	pub data: EmuReadyData<T>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmuReadyData<T> {
	pub json: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmuReadySystem {
	pub id: String,
	pub name: String,
	#[serde(default)]
	pub key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmuReadyGame {
	pub id: String,
	pub title: String,
	#[serde(rename = "normalizedTitle", default)]
	pub normalized_title: Option<String>,
	#[serde(rename = "systemId")]
	pub system_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmuReadyGamesPage {
	#[serde(default)]
	pub games: Vec<EmuReadyGame>,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn deserialises_systems_envelope_from_live_shape() {
		let body = r#"{
			"result": {
				"data": {
					"json": [
						{"id": "uuid-1", "name": "Nintendo Switch", "key": "nintendo_switch", "tgdbPlatformId": 4971},
						{"id": "uuid-2", "name": "Steam Deck", "key": "steam_deck"}
					]
				}
			}
		}"#;
		let env: EmuReadyEnvelope<Vec<EmuReadySystem>> = serde_json::from_str(body).unwrap();
		assert_eq!(env.result.data.json.len(), 2);
		assert_eq!(env.result.data.json[0].name, "Nintendo Switch");
		assert_eq!(
			env.result.data.json[0].key.as_deref(),
			Some("nintendo_switch")
		);
	}

	#[test]
	fn deserialises_games_get_envelope_from_live_shape() {
		let body = r#"{
			"result": {
				"data": {
					"json": {
						"games": [
							{
								"id": "game-uuid",
								"title": "Mario Kart 8 Deluxe",
								"normalizedTitle": "mario kart 8 deluxe",
								"systemId": "system-uuid",
								"imageUrl": "https://example.com/img.jpg",
								"_count": {"listings": 170}
							}
						]
					}
				}
			}
		}"#;
		let env: EmuReadyEnvelope<EmuReadyGamesPage> = serde_json::from_str(body).unwrap();
		let games = env.result.data.json.games;
		assert_eq!(games.len(), 1);
		assert_eq!(games[0].title, "Mario Kart 8 Deluxe");
		assert_eq!(
			games[0].normalized_title.as_deref(),
			Some("mario kart 8 deluxe")
		);
		assert_eq!(games[0].system_id, "system-uuid");
	}
}
