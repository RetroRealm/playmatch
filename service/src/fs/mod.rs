use std::path::{Path, PathBuf};

use async_recursion::async_recursion;
use tokio::fs;

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
