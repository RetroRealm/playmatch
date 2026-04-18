use super::download_and_extract_dats;
use reqwest::Client;

const REDUMP_NAME: &str = "redump";
const REDUMP_PRIVATE_NAME: &str = "redump-private";
const REDUMP_DOWNLOAD_URL: &str = "https://dats.retrorealm.dev/redump/daily";
const REDUMP_PRIVATE_DOWNLOAD_URL: &str = "https://dats.retrorealm.dev/redump/private";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedumpType {
	Public,
	Private,
}

fn get_redump_dat_name(r#type: RedumpType) -> &'static str {
	match r#type {
		RedumpType::Public => REDUMP_NAME,
		RedumpType::Private => REDUMP_PRIVATE_NAME,
	}
}

fn get_redump_download_url(r#type: RedumpType) -> &'static str {
	match r#type {
		RedumpType::Public => REDUMP_DOWNLOAD_URL,
		RedumpType::Private => REDUMP_PRIVATE_DOWNLOAD_URL,
	}
}

pub async fn download_redump_dats(client: &Client, r#type: RedumpType) -> anyhow::Result<()> {
	download_and_extract_dats(
		client,
		get_redump_dat_name(r#type),
		get_redump_download_url(r#type),
		false,
	)
	.await
}
