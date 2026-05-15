use std::path::{Path, PathBuf};

use crate::http::abstraction::RequestClientExt;
use futures_util::stream::StreamExt;
use lazy_static::lazy_static;
use log::{debug, warn};
use rand::distr::{Alphanumeric, SampleString};
use regex::Regex;
use reqwest::Client;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

lazy_static! {
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

	if let Some(content_disposition) = content_disposition
		&& let Ok(content_disposition) = content_disposition.to_str()
		&& let Some(filename) = extract_filename(content_disposition)
	{
		match sanitise_content_disposition_filename(&filename) {
			Some(safe) => {
				debug!(
					"Filename extracted from Content-Disposition header: {:?}",
					&safe
				);
				file_name = Some(safe);
			}
			None => {
				warn!(
					"Content-Disposition filename rejected as unsafe, falling back to random: {:?}",
					&filename
				);
			}
		}
	}

	let file_name_final = match &file_name {
		None => &Alphanumeric.sample_string(&mut rand::rng(), 16),
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

fn extract_filename(content_disposition: &str) -> Option<String> {
	if let Some(captures) = FILENAME_REGEX.captures(content_disposition) {
		return captures.get(1).map(|c| c.as_str().to_string());
	}
	None
}

/// Reject Content-Disposition filenames that could escape the target directory
/// (path separators, leading dot sequences) or are implausibly long.
/// Returns None on reject; caller falls back to a random name.
fn sanitise_content_disposition_filename(raw: &str) -> Option<String> {
	let trimmed = raw.trim().trim_matches('"').trim();
	if trimmed.is_empty() || trimmed.len() > 255 {
		return None;
	}
	if trimmed.contains('/') || trimmed.contains('\\') {
		return None;
	}
	if trimmed.starts_with('.') {
		return None;
	}
	Some(trimmed.to_owned())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn rejects_path_traversal() {
		assert_eq!(
			sanitise_content_disposition_filename("../../etc/passwd"),
			None
		);
		assert_eq!(
			sanitise_content_disposition_filename("..\\..\\secret"),
			None
		);
		assert_eq!(sanitise_content_disposition_filename("/abs/path"), None);
	}

	#[test]
	fn rejects_leading_dot() {
		assert_eq!(sanitise_content_disposition_filename(".hidden"), None);
		assert_eq!(sanitise_content_disposition_filename(".."), None);
	}

	#[test]
	fn rejects_empty_or_oversized() {
		assert_eq!(sanitise_content_disposition_filename(""), None);
		assert_eq!(sanitise_content_disposition_filename("   "), None);
		let long = "a".repeat(256);
		assert_eq!(sanitise_content_disposition_filename(&long), None);
	}

	#[test]
	fn accepts_normal_names() {
		assert_eq!(
			sanitise_content_disposition_filename("no-intro-snes.zip"),
			Some("no-intro-snes.zip".to_owned())
		);
		assert_eq!(
			sanitise_content_disposition_filename("  \"quoted name.dat\"  "),
			Some("quoted name.dat".to_owned())
		);
	}
}
