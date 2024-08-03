use log::debug;
use reqwest::Client;

use crate::http::download::download_file;

pub async fn download_redump_dats(client: &Client) -> anyhow::Result<()> {
	let response = client.get("http://redump.org/downloads/").send().await?;

	let body = response.text().await?;

	let html_parsed = scraper::Html::parse_document(&body);

	let selector = scraper::Selector::parse("#main > table > tbody > tr > td > a").unwrap();

	let mut dats_download_urls = vec![];

	for element in html_parsed.select(&selector) {
		let href = element.attr("href");

		if let Some(href) = href {
			if !href.contains("/datfile/") {
				continue;
			}

			dats_download_urls.push("http://redump.org".to_string() + href);
		}
	}

	for url in dats_download_urls {
		let current_dir = std::env::current_dir()?;
		debug!("Downloading DAT from: {}", url);
		let redump_dir = current_dir.join("dats/redump/");
		download_file(client, &url, &redump_dir).await?;
	}

	Ok(())
}
