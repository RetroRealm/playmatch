use log::{error, info};
use reqwest::Client;
use sea_orm::DbConn;
use serde::de::DeserializeOwned;
use service::dat::download_and_parse_dats;
use service::metadata::igdb::IgdbClient;
use service::r#match::igdb::match_db_to_igdb_entities;
use std::sync::Arc;
use tokio::task::JoinHandle;

pub async fn wrap_download_and_parse_dats(client: Arc<Client>, conn: Arc<DbConn>) {
	match download_and_parse_dats(client.as_ref(), conn.as_ref()).await {
		Ok(_) => {
			info!("Successfully downloaded and imported DATs");
		}
		Err(e) => {
			error!("Failed to download and imported DATs: {}", e);
		}
	}
}

pub async fn wrap_match_db_to_igdb_entities(igdb_client: Arc<IgdbClient>, conn: Arc<DbConn>) {
	match match_db_to_igdb_entities(igdb_client, &conn).await {
		Ok(()) => {
			info!("Successfully matched database to IGDB entities");
		}
		Err(err) => {
			error!("Failed to match database to IGDB entities: {}", err);
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
