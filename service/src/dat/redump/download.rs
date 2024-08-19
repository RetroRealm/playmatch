use crate::dat::shared::download::{delete_old_and_move_new_files, download_dat};
use crate::dat::shared::zip::extract_if_archived;
use crate::dat::DATS_PATH;
use anyhow::anyhow;
use log::error;
use reqwest::Client;
use tokio::fs;

const REDUMP_NAME: &str = "redump";
const DOWNLOAD_URL: &str = "https://dats.retrorealm.dev/redump/daily";

pub async fn download_redump_dats(client: &Client) -> anyhow::Result<()> {
	let current_dir = std::env::current_dir()?;
	let redump_dir = current_dir.join(DATS_PATH).join(REDUMP_NAME);
	let redump_tmp_dir = redump_dir.join("tmp");
	fs::create_dir_all(&redump_tmp_dir).await?;

	let path = download_dat(client, DOWNLOAD_URL, &redump_tmp_dir).await?;

	if let Err(e) = extract_if_archived(&path).await {
		error!("Failed to extract DAT archive {} {:?}", path.display(), e);
	}

	delete_old_and_move_new_files(&redump_dir, &redump_tmp_dir, false).await?;

	Ok(())
}
