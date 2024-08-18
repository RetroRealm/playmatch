use log::{error, info};
use reqwest::Client;
use sea_orm::DbConn;
use serde::de::DeserializeOwned;
use service::dat::download_and_parse_dats;
use std::sync::Arc;
use tokio::task::JoinHandle;

pub async fn download_and_parse_dats_wrapper(client: Arc<Client>, conn: Arc<DbConn>) {
	match download_and_parse_dats(client.as_ref(), conn.as_ref()).await {
		Ok(_) => {
			info!("Successfully downloaded and imported DATs");
		}
		Err(e) => {
			error!("Failed to download and imported DATs: {}", e);
		}
	}
}

pub async fn igdb_route_mutli_id_helper<T: DeserializeOwned>(
	ids: Vec<i32>,
	f: impl Fn(i32) -> JoinHandle<anyhow::Result<Option<T>>>,
) -> anyhow::Result<Vec<T>> {
	let mut requests = vec![];

	for id in ids {
		requests.push(f(id));
	}

	let mut response = vec![];

	for future in requests {
		if let Some(inner) = future.await?? {
			response.push(inner);
		}
	}

	Ok(response)
}
