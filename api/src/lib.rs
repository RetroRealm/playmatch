use std::env;

use actix_governor::{Governor, GovernorConfigBuilder};
use actix_web::middleware::{Compress, DefaultHeaders, Logger};
use actix_web::web::{scope, Data};
use actix_web::{App, HttpServer};
use dotenvy::dotenv;
use env_logger::Env;
use log::LevelFilter;
use sea_orm::{ConnectOptions, Database};
use utoipa::OpenApi;
use utoipa_swagger_ui::{SwaggerUi, Url};

use migration::{Migrator, MigratorTrait};
use service::dat::download_and_parse_dats;

use crate::models::game_file::GameMatchResponse;
use crate::models::game_file::GameMatchType;
use crate::routes::identify::__path_identify;
use crate::routes::identify::identify;

mod abstraction;
pub mod error;
mod models;
pub mod routes;

pub mod built_info {
    // The file has been placed there by the build script.
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

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
    // and replenishes two element every seconds
    let governor_conf = GovernorConfigBuilder::default()
        .use_headers()
        .per_millisecond(500)
        .burst_size(20)
        .finish()
        .unwrap();

    #[derive(OpenApi)]
    #[openapi(paths(identify), components(schemas(GameMatchResponse, GameMatchType)))]
    struct ApiDoc;

    let mut opt = ConnectOptions::new(env::var("DATABASE_URL")?);

    opt.sqlx_logging_level(LevelFilter::Debug);

    let conn = Database::connect(opt).await?;
    Migrator::up(&conn, None).await?;

    let client = reqwest::Client::builder().cookie_store(true).build()?;

    download_and_parse_dats(&client, &conn).await?;

    let conn_data = Data::new(conn);
    let client_data = Data::new(client);

    HttpServer::new(move || {
        App::new()
            .wrap(Compress::default())
            .app_data(conn_data.clone())
            .app_data(client_data.clone())
            .service(
                scope("/api")
                    .wrap(Governor::new(&governor_conf))
                    .wrap(Logger::default())
                    .wrap(DefaultHeaders::new().add(("X-Version", built_info::PKG_VERSION)))
                    .service(identify),
            )
            .service(SwaggerUi::new("/swagger-ui/{_:.*}").urls(vec![(
                Url::new("api", "/api-docs/openapi.json"),
                ApiDoc::openapi(),
            )]))
    })
    .bind(format!("0.0.0.0:{}", port))?
    .shutdown_timeout(15)
    .workers(worker_amount)
    .run()
    .await?;

    Ok(())
}

pub fn main() {
    let result = start();

    if let Some(err) = result.err() {
        println!("Error: {err}");
    }
}
