use crate::dat::shared::download;
use crate::dat::shared::zip::extract_if_archived;
use crate::dat::DATS_PATH;
use crate::util::random_sized_string;
use anyhow::anyhow;
use log::error;
use reqwest::Client;
use scraper::{Html, Selector};
use tokio::fs;
use tokio::task::JoinHandle;

const REDUMP_NAME: &str = "redump";
const REDUMP_URL: &str = "http://redump.org";

pub async fn download_redump_dats(client: &Client) -> anyhow::Result<()> {
	let dats_download_urls = get_redump_dat_download_urls(client).await?;

	let current_dir = std::env::current_dir()?;
	let redump_dir = current_dir.join(format!("{}/{}", DATS_PATH, REDUMP_NAME));
	let redump_tmp_dir = redump_dir.join("tmp");
	fs::create_dir_all(&redump_tmp_dir).await?;

	for url_chunk in dats_download_urls.chunks(5) {
		let mut tasks: Vec<JoinHandle<anyhow::Result<()>>> = vec![];
		for url in url_chunk {
			let redump_dir = redump_dir.clone();
			let tmp_dir = redump_dir.join(format!("tmp/{}", random_sized_string(16)));
			let client = client.clone();
			let url = url.to_string();
			tasks.push(tokio::spawn(async move {
				fs::create_dir_all(&tmp_dir).await?;
				let path = download::download_dat(&client, &url, &tmp_dir).await?;

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

	download::delete_old_and_move_new_files(&redump_dir, &redump_tmp_dir, false).await?;

	Ok(())
}

async fn get_redump_dat_download_urls(client: &Client) -> anyhow::Result<Vec<String>> {
	let response = client
		.get(format!("{}/downloads", REDUMP_URL))
		.send()
		.await?;

	let body = response.text().await?;

	let html_parsed = Html::parse_document(&body);

	let selector = Selector::parse("#main > table > tbody > tr > td > a")
		.map_err(|_| anyhow!("Couldn't parse the redump HTML"))?;

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
