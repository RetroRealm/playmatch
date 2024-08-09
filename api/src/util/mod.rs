use log::{error, info};
use reqwest::Client;
use sea_orm::DbConn;
use service::dat::download_and_parse_dats;
use std::sync::Arc;

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
