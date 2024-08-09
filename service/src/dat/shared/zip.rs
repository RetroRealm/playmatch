use crate::zip::extract_zip_to_directory;
use log::debug;
use std::path::{Path, PathBuf};
use tokio::fs;

pub async fn extract_if_archived(path: &PathBuf) -> anyhow::Result<()> {
    if let Some(file_extension) = path.extension() {
        let file_extension = file_extension.to_str().unwrap_or_default();
        if file_extension == "zip" {
            debug!("Found zip file, extracting...");
            extract_zip_in_same_path(path).await?;
            debug!("Removing zip file...");
            fs::remove_file(path).await?;
        }
    }

    Ok(())
}

async fn extract_zip_in_same_path(path: &PathBuf) -> anyhow::Result<()> {
    let out = path.parent().unwrap().join(path.file_stem().unwrap());
    debug!("Extracting DAT(s) to: {:?}", &out);
    let path_owned = path.to_owned();
    tokio::task::spawn_blocking(move || extract_zip_to_directory(&path_owned, Path::new(&out)))
        .await??;

    Ok(())
}
