use crate::providers::retroachievements::RetroAchievementsClient;
use crate::providers::retroachievements::model::{RaConsole, RaGameListEntry};
use anyhow::{Context, anyhow};
use reqwest::{StatusCode, Url};

/// Fetches every console RA knows about.
pub async fn get_console_ids(client: &RetroAchievementsClient) -> anyhow::Result<Vec<RaConsole>> {
	let url = build_url(client, "API_GetConsoleIDs.php", &[])?;
	let body = fetch_text(client, url, "get_console_ids").await?;
	let parsed: Vec<RaConsole> = serde_json::from_str(&body)
		.with_context(|| "failed to parse RetroAchievements console list")?;
	Ok(parsed)
}

/// Fetches the game list for one console, including every supported MD5
/// hash. `h=1` enables hash inclusion; `f=0` returns games regardless of
/// whether they currently have achievements.
pub async fn get_game_list(
	client: &RetroAchievementsClient,
	system_id: i32,
) -> anyhow::Result<Vec<RaGameListEntry>> {
	let url = build_url(
		client,
		"API_GetGameList.php",
		&[
			("i", system_id.to_string()),
			("h", "1".to_string()),
			("f", "0".to_string()),
		],
	)?;
	let body = fetch_text(client, url, "get_game_list").await?;
	let parsed: Vec<RaGameListEntry> = serde_json::from_str(&body).with_context(|| {
		format!("failed to parse RetroAchievements game list for system {system_id}")
	})?;
	Ok(parsed)
}

fn build_url(
	client: &RetroAchievementsClient,
	endpoint: &str,
	extra: &[(&'static str, String)],
) -> anyhow::Result<Url> {
	let mut url = Url::parse(&format!("{}/{endpoint}", client.api_base()))?;
	{
		let mut q = url.query_pairs_mut();
		q.append_pair("z", client.username());
		q.append_pair("y", client.api_key());
		for (k, v) in extra {
			q.append_pair(k, v);
		}
	}
	Ok(url)
}

async fn fetch_text(
	client: &RetroAchievementsClient,
	url: Url,
	endpoint_label: &'static str,
) -> anyhow::Result<String> {
	let response = client.http().get(url).send().await?;
	let status = response.status();
	let body = response.text().await?;
	match status {
		s if s.is_success() => Ok(body),
		StatusCode::TOO_MANY_REQUESTS => Err(anyhow!(
			"retroachievements {endpoint_label} rate-limited (HTTP 429)"
		)),
		s => Err(anyhow!(
			"retroachievements {endpoint_label} returned non-success status: {s} (body preview: {:?})",
			body.chars().take(200).collect::<String>()
		)),
	}
}
