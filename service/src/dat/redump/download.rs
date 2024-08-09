use std::path::{Path, PathBuf};

use log::{debug, error};
use reqwest::Client;
use scraper::{Html, Selector};
use tokio::fs;

use crate::dat::shared::zip::extract_if_archived;
use crate::dat::DATS_PATH;
use crate::fs::{read_files, read_files_recursive};
use crate::http::download::{download_file, DownloadFileNameResult};
use crate::util::random_sized_string;

const REDUMP_NAME: &str = "redump";
const REDUMP_URL: &str = "http://redump.org";

pub async fn download_redump_dats(client: &Client) -> anyhow::Result<()> {
    // TODO: make download and unzip happen in tmp dir, only delete files when all new files are downloaded to prevent data loss when redump is down or not reachable

    let dats_download_urls = get_redump_dat_download_urls(client).await?;

    let current_dir = std::env::current_dir()?;
    let redump_dir = current_dir.join(format!("{}/{}", DATS_PATH, REDUMP_NAME));
    let redump_tmp_dir = redump_dir.join("tmp");
    fs::create_dir_all(&redump_tmp_dir).await?;

    for url_chunk in dats_download_urls.chunks(5) {
        let mut tasks: Vec<tokio::task::JoinHandle<anyhow::Result<()>>> = vec![];
        for url in url_chunk {
            let redump_dir = redump_dir.clone();
            let tmp_dir = redump_dir.join(format!("tmp/{}", random_sized_string(16)));
            let client = client.clone();
            let url = url.to_string();
            tasks.push(tokio::spawn(async move {
                fs::create_dir_all(&tmp_dir).await?;
                let path = download_single_dat(&client, &url, &tmp_dir).await?;

                if let Err(e) = extract_if_archived(&path).await {
                    error!("Failed to extract DAT archive {} {:?}", path.display(), e);
                }

                Ok(())
            }));
        }
        for task in tasks {
            task.await??;
        }
    }

    let old_files = read_files(&redump_dir).await?;

    for old_file in &old_files {
        fs::remove_file(old_file).await?
    }

    let tmp_files = read_files_recursive(&redump_tmp_dir).await?;

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
            let out = redump_dir.join(file_name);
            fs::rename(&tmp_file, &out).await?;
        }
    }

    debug!("Removing redump tmp dir: {:?}", redump_tmp_dir);
    fs::remove_dir_all(&redump_tmp_dir).await?;

    Ok(())
}

async fn get_redump_dat_download_urls(client: &Client) -> anyhow::Result<Vec<String>> {
    let response = client
        .get(format!("{}/downloads", REDUMP_URL))
        .send()
        .await?;

    let body = response.text().await?;

    let html_parsed = Html::parse_document(&body);

    let selector = Selector::parse("#main > table > tbody > tr > td > a").unwrap();

    let mut dats_download_urls = vec![];

    for element in html_parsed.select(&selector) {
        let href = element.attr("href");

        if let Some(href) = href {
            if !href.contains("/datfile/") {
                continue;
            }

            dats_download_urls.push(format!("{}{}", REDUMP_URL, href));
        }
    }

    Ok(dats_download_urls)
}

async fn download_single_dat(
    client: &Client,
    url: &String,
    path: &Path,
) -> anyhow::Result<PathBuf> {
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
