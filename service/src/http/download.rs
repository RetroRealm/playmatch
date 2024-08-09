use std::path::{Path, PathBuf};

use crate::http::abstraction::RequestClientExt;
use crate::util::random_sized_string;
use futures_util::stream::StreamExt;
use lazy_static::lazy_static;
use log::debug;
use regex::Regex;
use reqwest::Client;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

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
	let response = client.get_default_user_agent(url).send().await?;
	let content_disposition = response.headers().get("content-disposition");

	let mut file_name = None;

	if let Some(content_disposition) = content_disposition {
		if let Ok(content_disposition) = content_disposition.to_str() {
			if let Some(filename) = extract_filename(content_disposition) {
				debug!(
					"Filename extracted from Content-Disposition header: {:?}",
					&filename
				);
				file_name = Some(filename);
			}
		}
	}

	let file_name_final = match &file_name {
		None => &random_sized_string(16),
		Some(file_name) => file_name,
	};

	let file_path = path.join(file_name_final);
	let mut file = File::create(&file_path).await?;
	let mut stream = response.bytes_stream();
	while let Some(v) = stream.next().await {
		file.write_buf(&mut v?).await?;
	}

	match file_name {
		Some(_) => Ok(DownloadFileNameResult::FromContentDisposition(file_path)),
		None => Ok(DownloadFileNameResult::Random(file_path)),
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
