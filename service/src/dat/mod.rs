use reqwest::Client;

use crate::dat::redump::download::download_redump_dats;

mod no_intro;
mod redump;

pub async fn download_and_parse_dats(client: &Client) -> anyhow::Result<()> {
	download_redump_dats(client).await?;

	Ok(())
}
