use crate::model::igdb::GameResponse;
use crate::routes::igdb::__path_get_game;
use actix_governor::{Governor, GovernorConfigBuilder};
use actix_web::middleware::{Compress, DefaultHeaders, Logger};
use actix_web::web::{scope, Data};
use actix_web::{App, HttpServer};
use dotenvy::dotenv;
use env_logger::Env;
use log::{debug, error, info, LevelFilter};
use reqwest::Client;
use sea_orm::{ConnectOptions, Database};
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};
use utoipa::OpenApi;
use utoipa_swagger_ui::{SwaggerUi, Url};

use migration::{Migrator, MigratorTrait};

use crate::routes::identify::__path_identify;
use crate::routes::identify::identify;
use crate::routes::igdb::get_game;
use crate::util::download_and_parse_dats_wrapper;
use model::game_file::GameMatchResponse;
use model::game_file::GameMatchType;
use service::metadata::igdb::IgdbClient;

pub mod error;
pub mod model;
pub mod routes;
mod util;

pub mod built_info {
	// The file has been placed there by the build script.
	include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[derive(OpenApi)]
#[openapi(
	paths(identify, get_game),
	components(schemas(GameMatchResponse, GameMatchType, GameResponse))
)]
struct ApiDoc;

#[actix_web::main]
async fn start() -> anyhow::Result<()> {
	// Load environment variables from .env file, if present but do nothing if it fails
	let _ = dotenv();
	env_logger::Builder::from_env(Env::default().default_filter_or("info")).try_init()?;

	let port = env::var("PORT").unwrap_or("8080".to_string());
	let worker_amount = match env::var("HTTP_WORKERS") {
		Ok(workers) => workers.parse::<usize>()?,
		Err(_) => num_cpus::get(),
	};

	// Allow bursts with up to 20 requests per IP address
	// and replenishes four element every seconds
	let governor_conf = GovernorConfigBuilder::default()
		.use_headers()
		.per_millisecond(250)
		.burst_size(20)
		.finish()
		.unwrap();

	let mut opt = ConnectOptions::new(env::var("DATABASE_URL")?);
	opt.sqlx_logging_level(LevelFilter::Debug);

	let conn = Database::connect(opt).await?;
	Migrator::up(&conn, None).await?;

	let sched = JobScheduler::new().await?;
	let client = Client::builder().cookie_store(true).build()?;
	let igdb_client = IgdbClient::new(
		env::var("IGDB_CLIENT_ID")?,
		env::var("IGDB_CLIENT_SECRET")?,
		client.clone(),
	)?;

	let conn_arc = Arc::new(conn);
	let client_arc = Arc::new(client);

	let conn_data = Data::from(conn_arc.clone());
	let client_data = Data::from(client_arc.clone());
	let igdb_data = Data::new(Mutex::new(igdb_client));

	let serv = HttpServer::new(move || {
		App::new()
			.wrap(Compress::default())
			.app_data(conn_data.clone())
			.app_data(client_data.clone())
			.app_data(igdb_data.clone())
			.service(
				scope("/api")
					.wrap(Governor::new(&governor_conf))
					.wrap(Logger::default())
					.wrap(DefaultHeaders::new().add(("X-Version", built_info::PKG_VERSION)))
					.service(identify)
					.service(get_game),
			)
			.service(SwaggerUi::new("/swagger-ui/{_:.*}").urls(vec![(
				Url::new("playmatch API", "/api-docs/openapi.json"),
				ApiDoc::openapi(),
			)]))
	})
	.bind(format!("0.0.0.0:{}", port))?
	.shutdown_timeout(15)
	.workers(worker_amount)
	.run();

	let conn = conn_arc.clone();
	let client = client_arc.clone();
	sched
		.add(Job::new_async("* * 12 * * *", move |_, _| {
			let conn = conn.clone();
			let client = client.clone();
			Box::pin(async move { download_and_parse_dats_wrapper(client, conn).await })
		})?)
		.await?;

	let conn = conn_arc.clone();
	let client = client_arc.clone();
	tokio::spawn(async move { download_and_parse_dats_wrapper(client, conn).await });

	sched.start().await?;
	debug!("Scheduler started");

	info!("Starting server on port {}", port);
	serv.await?;

	Ok(())
}

pub fn main() {
	let result = start();

	if let Some(err) = result.err() {
		println!("Error: {err}");
	}
}
