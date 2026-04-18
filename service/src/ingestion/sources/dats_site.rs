use super::download_and_extract_dats;
use reqwest::Client;

const DATS_SITE_LEGACY_NAME: &str = "dats-site-legacy";

const DATS_SITE_LEGACY_DOWNLOAD_URL: &str = "https://dats.retrorealm.dev/datssite/legacy";

pub async fn download_dats_site_legacy_dats(client: &Client) -> anyhow::Result<()> {
	download_and_extract_dats(
		client,
		DATS_SITE_LEGACY_NAME,
		DATS_SITE_LEGACY_DOWNLOAD_URL,
		false,
	)
	.await
}
