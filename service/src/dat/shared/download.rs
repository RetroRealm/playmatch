use crate::fs::{read_files, read_files_recursive};
use crate::http::download::{download_file, DownloadFileNameResult};
use log::debug;
use reqwest::Client;
use std::path::{Path, PathBuf};
use tokio::fs;

pub async fn download_dat(client: &Client, url: &str, path: &Path) -> anyhow::Result<PathBuf> {
	debug!("Downloading DAT from: {}", url);

	let (name_source, path) = match download_file(client, url, path).await? {
		DownloadFileNameResult::FromContentDisposition(path) => {
			(Some("Content-Disposition Header"), path)
		}
		DownloadFileNameResult::FromUrl(path) => (Some("URL path"), path),
		DownloadFileNameResult::Random(path) => (None, path),
	};

	let normalized_path = path.canonicalize()?;

	debug!(
		"Downloaded DAT from: {} to: {:?} (file name source: {})",
		url,
		normalized_path,
		name_source.unwrap_or("None")
	);

	Ok(normalized_path)
}

pub async fn delete_old_and_move_new_files(
	main_dir: &Path,
	tmp_dir: &Path,
	should_keep_subfolders: bool,
) -> anyhow::Result<()> {
	let main_dir = main_dir.canonicalize()?;
	let old_files = read_files(&main_dir).await?;

	for old_file in &old_files {
		fs::remove_file(old_file).await?
	}

	let tmp_files = read_files_recursive(tmp_dir).await?;

	debug!("Read {} temporary files", tmp_files.len());

	for tmp_file in tmp_files {
		let extension = tmp_file
			.extension()
			.unwrap_or_default()
			.to_str()
			.unwrap_or_default();
		let file_name = tmp_file
			.file_name()
			.unwrap_or_default()
			.to_str()
			.unwrap_or_default();

		if extension == "dat" {
			debug!("Moving DAT file: {:?}", tmp_file);

			let out = if should_keep_subfolders {
				let out = main_dir.join(tmp_file.strip_prefix(tmp_dir)?);
				fs::create_dir_all(out.parent().unwrap()).await?;
				out
			} else {
				main_dir.join(file_name)
			};

			fs::rename(&tmp_file, &out).await?;
		}
	}

	debug!("Removing tmp dir: {:?}", tmp_dir);
	fs::remove_dir_all(&tmp_dir).await?;
	Ok(())
}
