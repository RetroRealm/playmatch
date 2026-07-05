mod dats_site;
mod no_intro;
mod redump;

use crate::ingestion::archive::extract_if_archived;
use crate::ingestion::download::{delete_old_and_move_new_files, download_dat};
use crate::ingestion::{DATS_PATH, TMP_PATH};
use log::error;
use reqwest::Client;
use tokio::fs;

pub use self::dats_site::download_dats_site_legacy_dats;
pub use self::no_intro::download_no_intro_dats;
pub use self::redump::download_redump_dats;

pub(super) async fn download_and_extract_dats(
	client: &Client,
	name: &str,
	download_url: &str,
	keep_subfolders: bool,
) -> anyhow::Result<()> {
	let current_dir = std::env::current_dir()?;
	let dat_dir = current_dir.join(DATS_PATH);
	let tmp_dir = dat_dir.join(TMP_PATH).join(name);
	let parent_dir = dat_dir.join(name);
	fs::create_dir_all(&tmp_dir).await?;

	let path = download_dat(client, download_url, &tmp_dir).await?;

	if let Err(e) = extract_if_archived(&path).await {
		error!("Failed to extract DAT archive {} {:?}", path.display(), e);
	}

	delete_old_and_move_new_files(&parent_dir, &tmp_dir, keep_subfolders).await?;

	Ok(())
}
