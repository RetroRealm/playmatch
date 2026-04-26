use crate::middleware::user_agent_metric;
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
	get_igdb_character_by_id, get_igdb_character_gender_by_id, get_igdb_character_genders_by_ids,
	get_igdb_character_mug_shot_by_id, get_igdb_character_mug_shots_by_ids,
	get_igdb_character_species_by_id, get_igdb_character_species_by_ids,
	get_igdb_characters_by_ids, get_igdb_collection_by_id, get_igdb_collection_membership_by_id,
	get_igdb_collection_membership_type_by_id, get_igdb_collection_membership_types_by_ids,
	get_igdb_collection_memberships_by_ids, get_igdb_collection_relation_by_id,
	get_igdb_collection_relation_type_by_id, get_igdb_collection_relation_types_by_ids,
	get_igdb_collection_relations_by_ids, get_igdb_collection_type_by_id,
	get_igdb_collection_types_by_ids, get_igdb_collections_by_ids, get_igdb_companies_by_ids,
	get_igdb_company_by_id, get_igdb_company_logo_by_id, get_igdb_company_logos_by_ids,
	get_igdb_company_size_by_id, get_igdb_company_sizes_by_ids, get_igdb_company_status_by_id,
	get_igdb_company_statuses_by_ids, get_igdb_company_type_by_id,
	get_igdb_company_type_histories_by_ids, get_igdb_company_type_history_by_id,
	get_igdb_company_types_by_ids, get_igdb_company_website_by_id,
	get_igdb_company_websites_by_ids, get_igdb_cover_by_id, get_igdb_covers_by_ids,
	get_igdb_date_format_by_id, get_igdb_date_formats_by_ids, get_igdb_entity_type_by_id,
	get_igdb_entity_types_by_ids, get_igdb_event_by_id, get_igdb_event_logo_by_id,
	get_igdb_event_logos_by_ids, get_igdb_event_network_by_id, get_igdb_event_networks_by_ids,
	get_igdb_events_by_ids, get_igdb_external_game_by_id, get_igdb_external_game_source_by_id,
	get_igdb_external_game_sources_by_ids, get_igdb_external_games_by_ids,
	get_igdb_franchise_by_id, get_igdb_franchises_by_ids, get_igdb_game_by_id,
	get_igdb_game_engine_by_id, get_igdb_game_engine_logo_by_id, get_igdb_game_engine_logos_by_ids,
	get_igdb_game_engines_by_ids, get_igdb_game_localization_by_id,
	get_igdb_game_localizations_by_ids, get_igdb_game_mode_by_id, get_igdb_game_modes_by_ids,
	get_igdb_game_release_format_by_id, get_igdb_game_release_formats_by_ids,
	get_igdb_game_status_by_id, get_igdb_game_statuses_by_ids, get_igdb_game_time_to_beat_by_id,
	get_igdb_game_time_to_beats_by_ids, get_igdb_game_type_by_id, get_igdb_game_types_by_ids,
	get_igdb_game_version_by_id, get_igdb_game_version_feature_by_id,
	get_igdb_game_version_feature_value_by_id, get_igdb_game_version_feature_values_by_ids,
	get_igdb_game_version_features_by_ids, get_igdb_game_versions_by_ids,
	get_igdb_game_video_by_id, get_igdb_game_videos_by_ids, get_igdb_games_by_ids,
	get_igdb_genre_by_id, get_igdb_genres_by_ids, get_igdb_involved_companies_by_ids,
	get_igdb_involved_company_by_id, get_igdb_keyword_by_id, get_igdb_keywords_by_ids,
	get_igdb_language_by_id, get_igdb_language_support_by_id, get_igdb_language_support_type_by_id,
	get_igdb_language_support_types_by_ids, get_igdb_language_supports_by_ids,
	get_igdb_languages_by_ids, get_igdb_multiplayer_mode_by_id, get_igdb_multiplayer_modes_by_ids,
	get_igdb_network_type_by_id, get_igdb_network_types_by_ids, get_igdb_platform_by_id,
	get_igdb_platform_families_by_ids, get_igdb_platform_family_by_id,
	get_igdb_platform_logo_by_id, get_igdb_platform_logos_by_ids, get_igdb_platform_type_by_id,
	get_igdb_platform_types_by_ids, get_igdb_platform_version_by_id,
	get_igdb_platform_version_companies_by_ids, get_igdb_platform_version_company_by_id,
	get_igdb_platform_version_release_date_by_id, get_igdb_platform_version_release_dates_by_ids,
	get_igdb_platform_versions_by_ids, get_igdb_platform_website_by_id,
	get_igdb_platform_websites_by_ids, get_igdb_platforms_by_ids,
	get_igdb_player_perspective_by_id, get_igdb_player_perspectives_by_ids,
	get_igdb_popularity_primitive_by_id, get_igdb_popularity_primitives_by_ids,
	get_igdb_popularity_type_by_id, get_igdb_popularity_types_by_ids, get_igdb_region_by_id,
	get_igdb_regions_by_ids, get_igdb_release_date_by_id, get_igdb_release_date_region_by_id,
	get_igdb_release_date_regions_by_ids, get_igdb_release_date_status_by_id,
	get_igdb_release_date_statuses_by_ids, get_igdb_release_dates_by_ids, get_igdb_report_by_id,
	get_igdb_report_type_by_id, get_igdb_report_types_by_ids, get_igdb_reports_by_ids,
	get_igdb_screenshot_by_id, get_igdb_screenshots_by_ids, get_igdb_theme_by_id,
	get_igdb_themes_by_ids, get_igdb_website_by_id, get_igdb_website_type_by_id,
	get_igdb_website_types_by_ids, get_igdb_websites_by_ids, search_igdb_game_by_name,
};
use crate::routes::r#match::{
	manually_match_company, manually_match_game, manually_match_platform,
};
use crate::routes::platform::{get_all_platforms, get_platform_by_id};
use crate::routes::suggestion::{
	approve_suggestion, create_company_suggestion, create_game_suggestion,
	create_platform_suggestion, delete_suggestion, get_all_suggestions, get_suggestion_by_id,
	submit_external_game_suggestion,
};
use crate::routes::user::{
	create_or_get_by_discord_id, get_user, get_user_by_discord_id, update_user_permission_level,
};
use crate::util::{wrap_download_and_parse_dats, wrap_match_db_to_all_providers};
use actix_cors::Cors;
use actix_governor::{Governor, GovernorConfigBuilder};
use actix_web::middleware::{Compress, DefaultHeaders, Logger, from_fn};
use actix_web::web::{Data, JsonConfig, PayloadConfig, ServiceConfig, scope};
use actix_web::{App, HttpResponse, HttpServer, web};
use actix_web_prom::PrometheusMetricsBuilder;
use anyhow::anyhow;
use log::{Level, LevelFilter, debug, error, info};
use migration::{Migrator, MigratorTrait};
use prometheus::{Encoder, Registry, TextEncoder};
use reqwest::Client;
use sea_orm::{ConnectOptions, Database};
use service::config::http::X_VERSION_HEADER_API;
use service::db::constants::MAX_CONNECTIONS;
use service::providers::igdb::IgdbClient;
use service::providers::{MetadataProvider, ProviderRegistry};
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio_cron_scheduler::{Job, JobScheduler};
use util::http::ReverProxyExtractor;
use utoipa_swagger_ui::{SwaggerUi, Url};

pub mod error;
mod middleware;
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
	let sqlx_log_level = env::var("DATABASE_SQL_LOG_LEVEL")
		.ok()
		.and_then(|v| v.parse::<LevelFilter>().ok())
		.unwrap_or(LevelFilter::Debug);
	opt.sqlx_logging_level(sqlx_log_level);
	opt.sqlx_slow_statements_logging_settings(LevelFilter::Warn, Duration::from_secs(15));

	let conn = Database::connect(opt).await?;
	Migrator::up(&conn, None).await?;

	let sched = JobScheduler::new().await?;

	// Install the API key pepper before the server accepts requests. Missing or
	// malformed pepper is a loud startup panic.
	let pepper_raw = env::var("API_KEY_PEPPER").expect(
		"API_KEY_PEPPER environment variable is required (e.g. output of `openssl rand -hex 32`)",
	);
	service::db::user::init_pepper(&pepper_raw).unwrap_or_else(|e| panic!("API_KEY_PEPPER: {e}"));

	let igdb_http_client = Client::builder().cookie_store(true).build()?;
	let igdb_client = IgdbClient::new(
		env::var("IGDB_CLIENT_ID")?,
		env::var("IGDB_CLIENT_SECRET")?,
		igdb_http_client,
	)?;

	// DAT downloads use a cookieless client so hostile mirrors cannot set cookies that
	// would replay on subsequent requests to the same host.
	let dat_http_client = Client::builder().cookie_store(false).build()?;

	let redis_client = redis::Client::open(env::var("REDIS_URL")?)?;

	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	info!("Connected to Redis");

	let prometheus = PrometheusMetricsBuilder::new("api")
		.mask_unmatched_patterns("UNKNOWN")
		.exclude_regex(r"^/swagger-ui(/|$)")
		.build()
		.map_err(|e| anyhow!(e))?;

	service::metrics::init(&prometheus.registry)?;

	let metrics_registry = Data::new(prometheus.registry.clone());

	let conn_arc = Arc::new(conn);
	let dat_http_client_arc = Arc::new(dat_http_client);
	let igdb_client_arc = Arc::new(igdb_client);

	// Build the provider registry. Each constructed provider is registered as
	// `Arc<dyn MetadataProvider>` so the cron driver can dispatch uniformly.
	// Today only IGDB is wired; new providers slot in beside it.
	let providers: ProviderRegistry = vec![igdb_client_arc.clone() as Arc<dyn MetadataProvider>];
	let providers_arc = Arc::new(providers);

	let redis_client_data = Data::new(redis_client);
	let redis_conn_for_cron = redis_conn.clone();
	let redis_conn_data = Data::new(redis_conn);
	let conn_data = Data::from(conn_arc.clone());
	let igdb_data = Data::from(igdb_client_arc.clone());

	let serv = HttpServer::new(move || {
		App::new()
			.wrap(Compress::default())
			.wrap(prometheus.clone())
			.app_data(JsonConfig::default().limit(64 * 1024))
			.app_data(PayloadConfig::default().limit(256 * 1024))
			.app_data(conn_data.clone())
			.app_data(igdb_data.clone())
			.app_data(redis_client_data.clone())
			.app_data(redis_conn_data.clone())
			.service(
				scope("/api")
					.wrap(Governor::new(&governor_conf))
					.wrap(from_fn(user_agent_metric))
					.wrap(
						Logger::new("%{r}a %t \"%r\" %s %b \"%{User-Agent}i\" %T")
							.log_level(Level::Debug),
					)
					.wrap(
						DefaultHeaders::new()
							.add(("X-Version", X_VERSION_HEADER_API))
							.add((
								"Strict-Transport-Security",
								"max-age=31536000; includeSubDomains",
							))
							.add(("X-Content-Type-Options", "nosniff"))
							.add(("Referrer-Policy", "no-referrer"))
							.add(("Vary", "Origin")),
					)
					.wrap(Cors::permissive())
					.configure(configure_public_api_routes)
					.service(
						scope("")
							.wrap(
								DefaultHeaders::new()
									.add(("Cache-Control", "no-store"))
									.add(("Vary", "Authorization")),
							)
							.configure(configure_authenticated_api_routes),
					),
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
	let dat_client = dat_http_client_arc.clone();
	let providers_for_cron = providers_arc.clone();
	sched
		.add(Job::new_async("0 0 12 * * *", move |_, _| {
			let conn = conn.clone();
			let dat_client = dat_client.clone();
			let providers = providers_for_cron.clone();
			Box::pin(async move {
				wrap_download_and_parse_dats(dat_client, conn.clone(), false).await;
				wrap_match_db_to_all_providers(providers, conn.clone()).await;
			})
		})?)
		.await?;

	let external_conn = conn_arc.clone();
	let external_redis = redis_conn_for_cron;
	sched
		.add(Job::new_async("0/30 * * * * *", move |_, _| {
			let conn = external_conn.clone();
			let mut redis = external_redis.clone();
			Box::pin(async move {
				match service::external_suggestion::drain_external_suggestions(
					100, &conn, &mut redis,
				)
				.await
				{
					Ok(stats) if stats.processed_envelopes > 0 => {
						debug!(
							"external suggestion drain: processed {} envelopes (created: {}, already_matched: {}, duplicates: {}, unknown_roms: {}, invalid: {}, invalid_mappings: {}, unsupported_providers: {})",
							stats.processed_envelopes,
							stats.created,
							stats.already_matched,
							stats.duplicate_suggestions,
							stats.unknown_roms,
							stats.invalid_payloads,
							stats.invalid_mappings,
							stats.unsupported_providers,
						);
					}
					Ok(_) => {}
					Err(e) => error!("external suggestion drain failed: {e}"),
				}
			})
		})?)
		.await?;

	let conn = conn_arc.clone();
	let http_client = dat_http_client_arc.clone();
	let providers_for_init = providers_arc.clone();

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
			wrap_match_db_to_all_providers(providers_for_init, conn.clone()).await;
		});
	}

	sched.start().await?;
	debug!("Scheduler started");

	// Metrics run on a separate listener bound to an internal port so `/metrics` is never
	// exposed via the public listener.
	let metrics_port = env::var("METRICS_PORT").unwrap_or_else(|_| "9090".to_string());
	let metrics_serv = HttpServer::new(move || {
		App::new()
			.app_data(metrics_registry.clone())
			.route("/metrics", web::get().to(metrics_handler))
	})
	.bind(format!("0.0.0.0:{metrics_port}"))?
	.shutdown_timeout(5)
	.workers(1)
	.run();

	info!("Starting server on port {port}");
	info!("Starting metrics server on port {metrics_port}");
	tokio::try_join!(serv, metrics_serv)?;

	Ok(())
}

async fn metrics_handler(registry: Data<Registry>) -> HttpResponse {
	let encoder = TextEncoder::new();
	let mut buffer = Vec::new();
	if encoder.encode(&registry.gather(), &mut buffer).is_err() {
		return HttpResponse::InternalServerError().finish();
	}
	HttpResponse::Ok()
		.content_type(encoder.format_type())
		.body(buffer)
}

pub fn main() {
	let result = start();

	if let Some(err) = result.err() {
		println!("Error: {err}");
	}
}

fn configure_public_api_routes(cfg: &mut ServiceConfig) {
	cfg.service(health)
		.service(ready)
		.service(submit_external_game_suggestion)
		.service(get_all_companies)
		.service(get_company_by_id)
		.service(get_all_platforms)
		.service(get_platform_by_id)
		.service(identify_game_with_metadata_ids)
		.service(identify_game_and_relations)
		.service(get_playmatch_game_by_id)
		.service(get_playmatch_game_with_relations_by_id)
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
		.service(get_igdb_report_types_by_ids)
		.service(get_igdb_character_by_id)
		.service(get_igdb_characters_by_ids)
		.service(get_igdb_character_gender_by_id)
		.service(get_igdb_character_genders_by_ids)
		.service(get_igdb_character_species_by_id)
		.service(get_igdb_character_species_by_ids)
		.service(get_igdb_collection_membership_by_id)
		.service(get_igdb_collection_memberships_by_ids)
		.service(get_igdb_collection_membership_type_by_id)
		.service(get_igdb_collection_membership_types_by_ids)
		.service(get_igdb_collection_relation_by_id)
		.service(get_igdb_collection_relations_by_ids)
		.service(get_igdb_collection_relation_type_by_id)
		.service(get_igdb_collection_relation_types_by_ids)
		.service(get_igdb_collection_type_by_id)
		.service(get_igdb_collection_types_by_ids)
		.service(get_igdb_company_by_id)
		.service(get_igdb_companies_by_ids)
		.service(get_igdb_company_logo_by_id)
		.service(get_igdb_company_logos_by_ids)
		.service(get_igdb_company_website_by_id)
		.service(get_igdb_company_websites_by_ids)
		.service(get_igdb_event_by_id)
		.service(get_igdb_events_by_ids)
		.service(get_igdb_event_logo_by_id)
		.service(get_igdb_event_logos_by_ids)
		.service(get_igdb_event_network_by_id)
		.service(get_igdb_event_networks_by_ids)
		.service(get_igdb_game_engine_by_id)
		.service(get_igdb_game_engines_by_ids)
		.service(get_igdb_game_engine_logo_by_id)
		.service(get_igdb_game_engine_logos_by_ids)
		.service(get_igdb_game_localization_by_id)
		.service(get_igdb_game_localizations_by_ids)
		.service(get_igdb_game_mode_by_id)
		.service(get_igdb_game_modes_by_ids)
		.service(get_igdb_game_version_by_id)
		.service(get_igdb_game_versions_by_ids)
		.service(get_igdb_game_version_feature_by_id)
		.service(get_igdb_game_version_features_by_ids)
		.service(get_igdb_game_version_feature_value_by_id)
		.service(get_igdb_game_version_feature_values_by_ids)
		.service(get_igdb_game_video_by_id)
		.service(get_igdb_game_videos_by_ids)
		.service(get_igdb_involved_company_by_id)
		.service(get_igdb_involved_companies_by_ids)
		.service(get_igdb_keyword_by_id)
		.service(get_igdb_keywords_by_ids)
		.service(get_igdb_language_by_id)
		.service(get_igdb_languages_by_ids)
		.service(get_igdb_language_support_by_id)
		.service(get_igdb_language_supports_by_ids)
		.service(get_igdb_language_support_type_by_id)
		.service(get_igdb_language_support_types_by_ids)
		.service(get_igdb_multiplayer_mode_by_id)
		.service(get_igdb_multiplayer_modes_by_ids)
		.service(get_igdb_network_type_by_id)
		.service(get_igdb_network_types_by_ids)
		.service(get_igdb_platform_by_id)
		.service(get_igdb_platforms_by_ids)
		.service(get_igdb_platform_family_by_id)
		.service(get_igdb_platform_families_by_ids)
		.service(get_igdb_platform_logo_by_id)
		.service(get_igdb_platform_logos_by_ids)
		.service(get_igdb_platform_version_by_id)
		.service(get_igdb_platform_versions_by_ids)
		.service(get_igdb_platform_version_company_by_id)
		.service(get_igdb_platform_version_companies_by_ids)
		.service(get_igdb_platform_version_release_date_by_id)
		.service(get_igdb_platform_version_release_dates_by_ids)
		.service(get_igdb_platform_website_by_id)
		.service(get_igdb_platform_websites_by_ids)
		.service(get_igdb_player_perspective_by_id)
		.service(get_igdb_player_perspectives_by_ids)
		.service(get_igdb_popularity_primitive_by_id)
		.service(get_igdb_popularity_primitives_by_ids)
		.service(get_igdb_popularity_type_by_id)
		.service(get_igdb_popularity_types_by_ids)
		.service(get_igdb_region_by_id)
		.service(get_igdb_regions_by_ids)
		.service(get_igdb_release_date_by_id)
		.service(get_igdb_release_dates_by_ids)
		.service(get_igdb_release_date_status_by_id)
		.service(get_igdb_release_date_statuses_by_ids)
		.service(get_igdb_screenshot_by_id)
		.service(get_igdb_screenshots_by_ids)
		.service(get_igdb_theme_by_id)
		.service(get_igdb_themes_by_ids)
		.service(get_igdb_website_by_id)
		.service(get_igdb_websites_by_ids);
}

fn configure_authenticated_api_routes(cfg: &mut ServiceConfig) {
	cfg.service(manually_match_game)
		.service(manually_match_platform)
		.service(manually_match_company)
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
		.service(update_user_permission_level);
}
