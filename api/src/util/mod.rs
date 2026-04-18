pub mod http;

use log::{error, info};
use reqwest::Client;
use sea_orm::DbConn;
use serde::de::DeserializeOwned;
use service::ingestion::download_and_parse_dats;
use service::metrics::record_background_job;
use service::providers::igdb::IgdbClient;
use service::providers::igdb::matching::match_db_to_igdb_entities;
use std::sync::Arc;
use std::time::Instant;
use tokio::task::JoinHandle;

pub async fn wrap_download_and_parse_dats(
	client: Arc<Client>,
	conn: Arc<DbConn>,
	force_import: bool,
) {
	let started = Instant::now();
	let result = match download_and_parse_dats(client.as_ref(), conn.as_ref(), force_import).await {
		Ok(_) => {
			info!("Successfully downloaded and imported latest DATs");
			"success"
		}
		Err(e) => {
			error!("Failed to download and imported DATs: {e}");
			"failure"
		}
	};
	record_background_job("dat_ingest", result, started.elapsed().as_secs_f64());
}

pub async fn wrap_match_db_to_igdb_entities(igdb_client: Arc<IgdbClient>, conn: Arc<DbConn>) {
	let started = Instant::now();
	let result = match match_db_to_igdb_entities(igdb_client, &conn).await {
		Ok(()) => {
			info!("Successfully matched database to IGDB entities");
			"success"
		}
		Err(err) => {
			error!("Failed to match database to IGDB entities: {err}");
			"failure"
		}
	};
	record_background_job("igdb_match", result, started.elapsed().as_secs_f64());
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
