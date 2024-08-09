use anyhow::anyhow;
use entity::sea_orm_active_enums::GameReleaseProviderEnum;
use fs::read_files_recursive;
use log::{debug, error};
use reqwest::Client;
use sea_orm::DbConn;
use std::path::PathBuf;

use crate::dat::redump::download::download_redump_dats;
use crate::dat::shared::import::parse_and_import_dat_file;
use crate::fs;

mod no_intro;
mod redump;
pub mod shared;

const DATS_PATH: &str = "dats";

pub async fn download_and_parse_dats(client: &Client, conn: &DbConn) -> anyhow::Result<()> {
    download_redump_dats(client).await?;

    let files = read_files_recursive(&PathBuf::from(DATS_PATH)).await?;

    for file in files {
        let file_name = file
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();

        let extension = file
            .extension()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();

        let parent = file.to_str().unwrap_or_default();

        let mut provider = None;

        if parent.contains("no-intro") {
            provider = Some(GameReleaseProviderEnum::NoIntro)
        }

        if parent.contains("redump") {
            provider = Some(GameReleaseProviderEnum::Redump);
        }

        let provider = match provider {
            None => Err(anyhow!("Provider not found")),
            Some(provider) => Ok(provider),
        }?;

        if extension == "dat" && !file_name.contains("BIOS") {
            if let Err(e) = parse_and_import_dat_file(&file, provider, conn).await {
                error!("Failed to parse and import dat file: {:?}, {}", file, e);
            }
        }
    }

    Ok(())
}
