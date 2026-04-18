use super::download_and_extract_dats;
use reqwest::Client;

const NO_INTRO_NAME: &str = "no-intro";
const NO_INTRO_DOWNLOAD_URL: &str = "https://dats.retrorealm.dev/no-intro/daily";

pub async fn download_no_intro_dats(client: &Client) -> anyhow::Result<()> {
	download_and_extract_dats(client, NO_INTRO_NAME, NO_INTRO_DOWNLOAD_URL, true).await
}
