use super::download_and_extract_dats;
use reqwest::Client;

const REDUMP_NAME: &str = "redump";
const REDUMP_DOWNLOAD_URL: &str = "https://dats.retrorealm.dev/redump/daily";

pub async fn download_redump_dats(client: &Client) -> anyhow::Result<()> {
	download_and_extract_dats(client, REDUMP_NAME, REDUMP_DOWNLOAD_URL, false).await
}
