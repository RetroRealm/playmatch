use crate::dat::shared::download::{delete_old_and_move_new_files, download_dat};
use crate::dat::shared::zip::extract_if_archived;
use crate::dat::{DATS_PATH, TMP_PATH};
use log::error;
use reqwest::Client;
use tokio::fs;

const NO_INTRO_NAME: &str = "no-intro";
const DOWNLOAD_URL: &str = "https://dats.retrorealm.dev/no-intro/daily";

#[allow(dead_code)]
pub async fn download_no_intro_dats(client: &Client) -> anyhow::Result<()> {
	let current_dir = std::env::current_dir()?;
	let dat_dir = current_dir.join(DATS_PATH);
	let no_intro_tmp_dir = dat_dir.join(TMP_PATH).join(NO_INTRO_NAME);
	let no_intro_dir = dat_dir.join(NO_INTRO_NAME);
	fs::create_dir_all(&no_intro_tmp_dir).await?;

	let path = download_dat(client, DOWNLOAD_URL, &no_intro_tmp_dir).await?;

	if let Err(e) = extract_if_archived(&path).await {
		error!("Failed to extract DAT archive {} {:?}", path.display(), e);
	}

	delete_old_and_move_new_files(&no_intro_dir, &no_intro_tmp_dir, true).await?;

	Ok(())
}
