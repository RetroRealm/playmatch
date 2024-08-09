use std::path::{Path, PathBuf};

use futures_util::stream::StreamExt;
use lazy_static::lazy_static;
use log::debug;
use regex::Regex;
use reqwest::Client;
use tempdir::TempDir;
use tokio::fs;
use tokio::fs::{create_dir_all, File};
use tokio::io::AsyncWriteExt;

use crate::util::random_sized_string;

lazy_static! {
    // Define the regex pattern for extracting the filename
    static ref FILENAME_REGEX: Regex = Regex::new(r#"filename\*?=(?:UTF-8''|")?([^";]+)"#).unwrap();
}

#[derive(Debug)]
pub enum DownloadFileNameResult {
    FromContentDisposition(PathBuf),
    FromUrl(PathBuf),
    Random(PathBuf),
}

pub async fn download_file(
    client: &Client,
    url: &str,
    path: &Path,
) -> anyhow::Result<DownloadFileNameResult> {
    let mut path = PathBuf::from(path);
    let tmp_path = path.join("tmp");
    fs::create_dir_all(&tmp_path).await?;
    let response = client.get(url).send().await?;
    let content_disposition = response.headers().get("content-disposition");

    let mut found_filename_in_content_disposition = false;

    if let Some(content_disposition) = content_disposition {
        if let Ok(content_disposition) = content_disposition.to_str() {
            if let Some(filename) = extract_filename(content_disposition) {
                debug!(
                    "Filename extracted from Content-Disposition header: {:?}",
                    &filename
                );
                path.push(&filename);
                found_filename_in_content_disposition = true;
            }
        }
    }

    let tmp_dir = TempDir::new_in(&tmp_path, random_sized_string(15).as_str())?;
    let tmp_dir_path = tmp_dir
        .path()
        .join(path.file_name().unwrap().to_str().unwrap());
    let mut file = File::create(&tmp_dir_path).await?;
    let mut stream = response.bytes_stream();
    while let Some(v) = stream.next().await {
        file.write_buf(&mut v?).await?;
    }

    create_dir_all(path.parent().unwrap()).await?;
    fs::rename(&tmp_dir_path, &path).await?;

    if found_filename_in_content_disposition {
        Ok(DownloadFileNameResult::FromContentDisposition(path))
    } else {
        Ok(DownloadFileNameResult::Random(path))
    }
}

// Function to extract the filename from Content-Disposition header value
fn extract_filename(content_disposition: &str) -> Option<String> {
    // Using the cached regex to capture the filename part
    if let Some(captures) = FILENAME_REGEX.captures(content_disposition) {
        return captures.get(1).map(|c| c.as_str().to_string());
    }
    None
}
