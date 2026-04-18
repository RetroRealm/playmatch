use crate::openapi::create_openapi;
use crate::routes::company::{get_all_companies, get_company_by_id};
use crate::routes::game::{get_playmatch_game_by_id, get_playmatch_game_with_relations_by_id};
use crate::routes::health::{health, ready};
use crate::routes::identify::{identify_game_and_relations, identify_game_with_metadata_ids};
use crate::routes::igdb::{
	get_igdb_age_rating_by_id, get_igdb_age_rating_categories_by_ids,
	get_igdb_age_rating_category_by_id, get_igdb_age_rating_content_description_type_by_id,
	get_igdb_age_rating_content_description_types_by_ids,
	get_igdb_age_rating_content_description_v2_by_id,
	get_igdb_age_rating_content_descriptions_v2_by_ids, get_igdb_age_rating_organization_by_id,
	get_igdb_age_rating_organizations_by_ids, get_igdb_age_ratings_by_ids,
	get_igdb_alternative_name_by_id, get_igdb_alternative_names_by_ids, get_igdb_artwork_by_id,
	get_igdb_artwork_type_by_id, get_igdb_artwork_types_by_ids, get_igdb_artworks_by_ids,
	get_igdb_character_mug_shot_by_id, get_igdb_character_mug_shots_by_ids,
	get_igdb_collection_by_id, get_igdb_collections_by_ids, get_igdb_company_size_by_id,
	get_igdb_company_sizes_by_ids, get_igdb_company_status_by_id, get_igdb_company_statuses_by_ids,
	get_igdb_company_type_by_id, get_igdb_company_type_histories_by_ids,
	get_igdb_company_type_history_by_id, get_igdb_company_types_by_ids, get_igdb_cover_by_id,
	get_igdb_covers_by_ids, get_igdb_date_format_by_id, get_igdb_date_formats_by_ids,
	get_igdb_entity_type_by_id, get_igdb_entity_types_by_ids, get_igdb_external_game_by_id,
	get_igdb_external_game_source_by_id, get_igdb_external_game_sources_by_ids,
	get_igdb_external_games_by_ids, get_igdb_franchise_by_id, get_igdb_franchises_by_ids,
	get_igdb_game_by_id, get_igdb_game_release_format_by_id, get_igdb_game_release_formats_by_ids,
	get_igdb_game_status_by_id, get_igdb_game_statuses_by_ids, get_igdb_game_time_to_beat_by_id,
	get_igdb_game_time_to_beats_by_ids, get_igdb_game_type_by_id, get_igdb_game_types_by_ids,
	get_igdb_games_by_ids, get_igdb_genre_by_id, get_igdb_genres_by_ids,
	get_igdb_platform_type_by_id, get_igdb_platform_types_by_ids,
	get_igdb_release_date_region_by_id, get_igdb_release_date_regions_by_ids,
	get_igdb_report_by_id, get_igdb_report_type_by_id, get_igdb_report_types_by_ids,
	get_igdb_reports_by_ids, get_igdb_website_type_by_id, get_igdb_website_types_by_ids,
	search_igdb_game_by_name,
};
use crate::routes::r#match::{
	manually_match_company, manually_match_game, manually_match_platform,
};
use crate::routes::platform::{get_all_platforms, get_platform_by_id};
use crate::routes::suggestion::{
	approve_suggestion, create_company_suggestion, create_game_suggestion,
	create_platform_suggestion, delete_suggestion, get_all_suggestions, get_suggestion_by_id,
};
use crate::routes::user::{
	create_or_get_by_discord_id, get_user, get_user_by_discord_id, update_user_permission_level,
};
use crate::util::{wrap_download_and_parse_dats, wrap_match_db_to_igdb_entities};
use actix_governor::{Governor, GovernorConfigBuilder};
use actix_web::middleware::{Compress, DefaultHeaders, Logger};
use actix_web::web::{Data, ServiceConfig, scope};
use actix_web::{App, HttpServer};
use actix_web_prom::PrometheusMetricsBuilder;
use anyhow::anyhow;
use log::{Level, LevelFilter, debug, error, info};
use migration::{Migrator, MigratorTrait};
use reqwest::Client;
use sea_orm::{ConnectOptions, Database};
use service::config::http::X_VERSION_HEADER_API;
use service::db::constants::MAX_CONNECTIONS;
use service::providers::igdb::IgdbClient;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio_cron_scheduler::{Job, JobScheduler};
use util::http::ReverProxyExtractor;
use utoipa_swagger_ui::{SwaggerUi, Url};

pub mod error;
pub mod model;
mod openapi;
pub mod routes;
mod util;

#[actix_web::main]
async fn start() -> anyhow::Result<()> {
	let port = env::var("PORT").unwrap_or("8080".to_string());
	let worker_amount = match env::var("HTTP_WORKERS") {
		Ok(workers) => workers.parse::<usize>()?,
		Err(_) => *service::config::CPU_COUNT,
	};

	// Allow bursts with up to 20 requests per IP address
	// and replenishes four elements every second
	let governor_conf = GovernorConfigBuilder::default()
		.use_headers()
		.milliseconds_per_request(250)
		.key_extractor(ReverProxyExtractor)
		.burst_size(20)
		.finish()
		.expect("governor config is valid by construction");

	let mut opt = ConnectOptions::new(env::var("DATABASE_URL")?);
	opt.max_connections(
		env::var("DATABASE_MAX_CONNECTIONS")
			.unwrap_or(MAX_CONNECTIONS.to_string())
			.parse::<u32>()?,
	);
	opt.sqlx_logging_level(LevelFilter::Debug);
	opt.sqlx_slow_statements_logging_settings(LevelFilter::Warn, Duration::from_secs(15));

	let conn = Database::connect(opt).await?;
	Migrator::up(&conn, None).await?;

	let sched = JobScheduler::new().await?;
	let client = Client::builder().cookie_store(true).build()?;
	let igdb_client = IgdbClient::new(
		env::var("IGDB_CLIENT_ID")?,
		env::var("IGDB_CLIENT_SECRET")?,
		client.clone(),
	)?;
	let redis_client = redis::Client::open(env::var("REDIS_URL")?)?;

	redis_client.get_multiplexed_async_connection().await?;
	info!("Connected to Redis");

	let prometheus = PrometheusMetricsBuilder::new("api")
		.endpoint("/metrics")
		.build()
		.map_err(|e| anyhow!(e))?;

	let conn_arc = Arc::new(conn);
	let client_arc = Arc::new(client);
	let igdb_client_arc = Arc::new(igdb_client);

	let redis_client_data = Data::new(redis_client);
	let conn_data = Data::from(conn_arc.clone());
	let client_data = Data::from(client_arc.clone());
	let igdb_data = Data::from(igdb_client_arc.clone());

	let serv = HttpServer::new(move || {
		App::new()
			.wrap(Compress::default())
			.wrap(prometheus.clone())
			.app_data(conn_data.clone())
			.app_data(client_data.clone())
			.app_data(igdb_data.clone())
			.app_data(redis_client_data.clone())
			.service(
				scope("/api")
					.wrap(Governor::new(&governor_conf))
					.wrap(
						Logger::new("%{r}a %t \"%r\" %s %b \"%{Referer}i\" \"%{User-Agent}i\" %T")
							.log_level(Level::Debug),
					)
					.wrap(DefaultHeaders::new().add(("X-Version", X_VERSION_HEADER_API.clone())))
					.configure(configure_api_routes),
			)
			.service(SwaggerUi::new("/swagger-ui/{_:.*}").urls(vec![(
				Url::new("playmatch API", "/api-docs/openapi.json"),
				create_openapi(),
			)]))
	})
	.bind(format!("0.0.0.0:{port}"))?
	.shutdown_timeout(15)
	.workers(worker_amount)
	.run();

	let conn = conn_arc.clone();
	let client = client_arc.clone();
	let igdb_client = igdb_client_arc.clone();
	sched
		.add(Job::new_async("0 0 12 * * *", move |_, _| {
			let conn = conn.clone();
			let client = client.clone();
			let igdb_client = igdb_client.clone();
			Box::pin(async move {
				wrap_download_and_parse_dats(client, conn.clone(), false).await;
				wrap_match_db_to_igdb_entities(igdb_client, conn.clone()).await;
			})
		})?)
		.await?;

	let conn = conn_arc.clone();
	let http_client = client_arc.clone();
	let igdb_client = igdb_client_arc.clone();

	let initial_data_init = env::var("INITIAL_DATA_INIT")
		.unwrap_or("true".to_string())
		.to_lowercase()
		== "true";

	let force_initial_data_init = env::var("FORCE_INITIAL_DATA_INIT")
		.unwrap_or("false".to_string())
		.to_lowercase()
		== "true";

	if initial_data_init {
		tokio::spawn(async move {
			wrap_download_and_parse_dats(http_client, conn.clone(), force_initial_data_init).await;
			wrap_match_db_to_igdb_entities(igdb_client, conn.clone()).await;
		});
	}

	sched.start().await?;
	debug!("Scheduler started");

	info!("Starting server on port {port}");
	serv.await?;

	Ok(())
}

pub fn main() {
	let result = start();

	if let Some(err) = result.err() {
		println!("Error: {err}");
	}
}

fn configure_api_routes(cfg: &mut ServiceConfig) {
	cfg.service(health)
		.service(ready)
		.service(get_all_companies)
		.service(get_company_by_id)
		.service(get_all_platforms)
		.service(get_platform_by_id)
		.service(identify_game_with_metadata_ids)
		.service(identify_game_and_relations)
		.service(manually_match_game)
		.service(manually_match_platform)
		.service(manually_match_company)
		.service(get_playmatch_game_by_id)
		.service(get_playmatch_game_with_relations_by_id)
		.service(get_suggestion_by_id)
		.service(get_all_suggestions)
		.service(create_game_suggestion)
		.service(create_company_suggestion)
		.service(create_platform_suggestion)
		.service(approve_suggestion)
		.service(delete_suggestion)
		.service(create_or_get_by_discord_id)
		.service(get_user_by_discord_id)
		.service(get_user)
		.service(update_user_permission_level)
		.service(get_igdb_game_by_id)
		.service(get_igdb_games_by_ids)
		.service(search_igdb_game_by_name)
		.service(get_igdb_age_rating_by_id)
		.service(get_igdb_age_ratings_by_ids)
		.service(get_igdb_alternative_name_by_id)
		.service(get_igdb_alternative_names_by_ids)
		.service(get_igdb_artwork_by_id)
		.service(get_igdb_artworks_by_ids)
		.service(get_igdb_collection_by_id)
		.service(get_igdb_collections_by_ids)
		.service(get_igdb_cover_by_id)
		.service(get_igdb_covers_by_ids)
		.service(get_igdb_external_game_by_id)
		.service(get_igdb_external_games_by_ids)
		.service(get_igdb_franchise_by_id)
		.service(get_igdb_franchises_by_ids)
		.service(get_igdb_genre_by_id)
		.service(get_igdb_genres_by_ids)
		.service(get_igdb_age_rating_category_by_id)
		.service(get_igdb_age_rating_categories_by_ids)
		.service(get_igdb_age_rating_content_description_v2_by_id)
		.service(get_igdb_age_rating_content_descriptions_v2_by_ids)
		.service(get_igdb_age_rating_content_description_type_by_id)
		.service(get_igdb_age_rating_content_description_types_by_ids)
		.service(get_igdb_age_rating_organization_by_id)
		.service(get_igdb_age_rating_organizations_by_ids)
		.service(get_igdb_company_status_by_id)
		.service(get_igdb_company_statuses_by_ids)
		.service(get_igdb_date_format_by_id)
		.service(get_igdb_date_formats_by_ids)
		.service(get_igdb_external_game_source_by_id)
		.service(get_igdb_external_game_sources_by_ids)
		.service(get_igdb_game_release_format_by_id)
		.service(get_igdb_game_release_formats_by_ids)
		.service(get_igdb_game_status_by_id)
		.service(get_igdb_game_statuses_by_ids)
		.service(get_igdb_game_type_by_id)
		.service(get_igdb_game_types_by_ids)
		.service(get_igdb_platform_type_by_id)
		.service(get_igdb_platform_types_by_ids)
		.service(get_igdb_release_date_region_by_id)
		.service(get_igdb_release_date_regions_by_ids)
		.service(get_igdb_website_type_by_id)
		.service(get_igdb_website_types_by_ids)
		.service(get_igdb_artwork_type_by_id)
		.service(get_igdb_artwork_types_by_ids)
		.service(get_igdb_character_mug_shot_by_id)
		.service(get_igdb_character_mug_shots_by_ids)
		.service(get_igdb_company_size_by_id)
		.service(get_igdb_company_sizes_by_ids)
		.service(get_igdb_company_type_by_id)
		.service(get_igdb_company_types_by_ids)
		.service(get_igdb_company_type_history_by_id)
		.service(get_igdb_company_type_histories_by_ids)
		.service(get_igdb_entity_type_by_id)
		.service(get_igdb_entity_types_by_ids)
		.service(get_igdb_game_time_to_beat_by_id)
		.service(get_igdb_game_time_to_beats_by_ids)
		.service(get_igdb_report_by_id)
		.service(get_igdb_reports_by_ids)
		.service(get_igdb_report_type_by_id)
		.service(get_igdb_report_types_by_ids);
}
