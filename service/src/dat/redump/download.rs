use std::path::Path;

use log::debug;
use reqwest::Client;
use scraper::{Html, Selector};
use tokio::fs;

use crate::dat::DATS_PATH;
use crate::http::download::{download_file, DownloadFileNameResult};
use crate::zip::extract_zip_to_directory;

const REDUMP_URL: &str = "http://redump.org";

pub async fn download_redump_dats(client: &Client) -> anyhow::Result<()> {
    // TODO: make download and unzip happen in tmp dir, only delete files when all new files are downloaded to prevent data loss when redump is down or not reachable

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

    for url_chunk in dats_download_urls.chunks(5) {
        let mut tasks = vec![];
        for url in url_chunk {
            let client = client.clone();
            let url = url.to_string();
            tasks.push(tokio::spawn(async move {
                download_single_dat(&client, &url).await
            }));
        }
        for task in tasks {
            task.await??;
        }
    }

    Ok(())
}

async fn download_single_dat(client: &Client, url: &String) -> anyhow::Result<()> {
    let current_dir = std::env::current_dir()?;
    debug!("Downloading DAT from: {}", url);
    let redump_dir = current_dir.join(format!("{}/redump", DATS_PATH));
    let res = download_file(client, &url, &redump_dir).await?;
    let (name_source, path) = match res {
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

    if let Some(file_extension) = normalized_path.extension() {
        let file_extension = file_extension.to_str().unwrap();
        if file_extension == "zip" {
            debug!("Found zip file, extracting...");
            let out = normalized_path
                .parent()
                .unwrap()
                .join(normalized_path.file_stem().unwrap().to_owned());
            debug!("Extracting DAT to: {:?}", &out);
            let path_owned = normalized_path.to_owned();
            tokio::task::spawn_blocking(move || {
                extract_zip_to_directory(&path_owned, Path::new(&out))
            })
            .await??;
            fs::remove_file(&path).await?;
        }
    }
    Ok(())
}
