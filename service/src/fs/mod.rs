use std::path::{Path, PathBuf};

use async_recursion::async_recursion;
use fs::File;
use md5::{Digest, Md5};
use tokio::fs;
use tokio::io::AsyncReadExt;

pub async fn calculate_md5(path: &Path) -> anyhow::Result<String> {
	let mut file = File::open(path).await?;
	let mut hasher = Md5::new();

	let mut buffer = [0; 1024];
	loop {
		let n = file.read(&mut buffer).await?;
		if n == 0 {
			break;
		}
		hasher.update(&buffer[..n]);
	}

	Ok(format!("{:x}", hasher.finalize()))
}

#[async_recursion]
pub async fn read_files_recursive(folder_path: &Path) -> anyhow::Result<Vec<PathBuf>> {
	let mut dir = fs::read_dir(folder_path).await?;
	let mut files = Vec::new();

	while let Some(entry) = dir.next_entry().await? {
		let path = entry.path();

		if path.is_dir() {
			files.append(&mut read_files_recursive(&path).await?);
		} else {
			files.push(path);
		}
	}

	Ok(files)
}

pub async fn read_files(path_: &Path) -> anyhow::Result<Vec<PathBuf>> {
	let mut dir = fs::read_dir(path_).await?;
	let mut files = Vec::new();

	while let Some(entry) = dir.next_entry().await? {
		let path = entry.path();

		if path.is_file() {
			files.push(path);
		}
	}

	Ok(files)
}
