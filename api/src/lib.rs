use crate::middleware::{http_request_metrics, v2_error_envelope, version_lifecycle_headers};
#[doc(hidden)]
pub use crate::openapi::{create_openapi, create_openapi_v1, create_openapi_v2};
use crate::routes::company::{get_all_companies, get_company_by_id};
use crate::routes::game::{
	get_game_file_history_by_id, get_playmatch_game_by_id, get_playmatch_game_with_relations_by_id,
	search_games,
};
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
use crate::routes::launchbox::{
	get_lb_game_alternate_names, get_lb_game_by_id, get_lb_game_images, list_lb_platforms,
	search_lb_games,
};
use crate::routes::r#match::{
	manually_match_company, manually_match_game, manually_match_platform,
};
use crate::routes::mobygames::{
	get_mg_game_by_id, get_mg_game_covers, get_mg_game_screenshots, list_mg_genres,
	list_mg_platforms, search_mg_games,
};
use crate::routes::openvgdb::{get_ovgdb_release_by_id, get_ovgdb_rom_by_hash};
use crate::routes::platform::{get_all_platforms, get_platform_by_id};
use crate::routes::retroachievements::{
	get_ra_game_by_hash, get_ra_game_by_id, list_ra_systems, search_ra_games,
};
use crate::routes::screenscraper::{
	get_ss_game_by_id, get_ss_game_by_rom_name, list_ss_systems, search_ss_games,
};
use crate::routes::sgdb::{
	get_sgdb_game_by_id, get_sgdb_game_by_platform, get_sgdb_grids_by_game,
	get_sgdb_grids_by_platform, get_sgdb_heroes_by_game, get_sgdb_heroes_by_platform,
	get_sgdb_icons_by_game, get_sgdb_icons_by_platform, get_sgdb_logos_by_game,
	get_sgdb_logos_by_platform, search_sgdb_games,
};
use crate::routes::signature_group::{get_all_signature_groups, get_signature_group_by_id};
use crate::routes::suggestion::{
	approve_suggestion, create_company_suggestion, create_game_suggestion,
	create_platform_suggestion, delete_suggestion, get_all_suggestions, get_suggestion_by_id,
	submit_external_game_suggestion,
};
use crate::routes::user::{
	create_or_get_by_discord_id, get_user, get_user_by_discord_id, update_user_permission_level,
};
use crate::routes::v2::bulk_by_id::{
	bulk_companies_by_id_v2, bulk_dat_files_by_id_v2, bulk_game_files_by_id_v2,
	bulk_games_by_id_v2, bulk_platforms_by_id_v2, bulk_signature_groups_by_id_v2,
};
use crate::routes::v2::company::{get_company_by_id_v2, list_companies_v2, search_companies_v2};
use crate::routes::v2::dat_file::{
	get_dat_file_by_id_v2, get_dat_file_import_v2, list_dat_file_games_v2,
	list_dat_file_imports_v2, list_dat_files_by_hash_v2, list_dat_files_v2, list_game_dat_files_v2,
};
use crate::routes::v2::game::{
	get_game_by_id_v2, get_game_file_by_id_v2, get_game_file_history_by_id_v2,
	get_game_file_presence_v2, get_game_with_relations_by_id_v2, get_games_by_name_v2,
	list_game_clones_v2, list_game_files_v2, list_game_mappings_v2, list_games_v2, search_games_v2,
};
use crate::routes::v2::identify::{
	identify_game_and_relations_v2, identify_game_with_metadata_ids_v2,
};
use crate::routes::v2::identify_bulk::{identify_bulk_ids_v2, identify_bulk_relations_v2};
use crate::routes::v2::platform::{get_platform_by_id_v2, list_platforms_v2, search_platforms_v2};
use crate::routes::v2::signature_group::{
	get_signature_group_by_id_v2, list_signature_group_dat_files_v2, list_signature_group_games_v2,
	list_signature_groups_v2, search_signature_groups_v2,
};
use crate::routes::v2::stats::{get_platform_stats_v2, get_stats_v2};
use crate::routes::v2::suggestion::list_suggestions_v2;
use crate::routes::v2::user::{
	create_or_get_by_discord_id_v2, get_user_by_discord_id_v2, update_user_permission_level_v2,
};
use crate::routes::versions::{ResolvedDefaultVersion, get_api_versions};
use crate::util::{
	wrap_download_and_parse_dats, wrap_launchbox_import, wrap_match_db_to_all_providers,
	wrap_openvgdb_import, wrap_retroachievements_import,
};
use actix_cors::Cors;
use actix_governor::governor::middleware::StateInformationMiddleware;
use actix_governor::{Governor, GovernorConfig, GovernorConfigBuilder};
use actix_web::dev::HttpServiceFactory;
use actix_web::http::Method;
use actix_web::http::header;
use actix_web::middleware::{Compress, DefaultHeaders, Logger, from_fn};
use actix_web::web::{Data, JsonConfig, PayloadConfig, ServiceConfig, scope};
use actix_web::{App, HttpResponse, HttpServer, web};
use actix_web_prom::PrometheusMetricsBuilder;
use anyhow::anyhow;
use log::{Level, LevelFilter, debug, error, info, warn};
use migration::{Migrator, MigratorTrait};
use prometheus::{Encoder, Registry, TextEncoder};
use reqwest::Client;
use sea_orm::{ConnectOptions, Database};
use service::config::http::X_VERSION_HEADER_API;
use service::config::versions::ApiVersion;
use service::db::constants::MAX_CONNECTIONS;
use service::providers::emuready::EmuReadyClient;
use service::providers::hasheous::HasheousClient;
use service::providers::igdb::IgdbClient;
use service::providers::launchbox::LaunchBoxClient;
use service::providers::mobygames::MobyGamesClient;
use service::providers::openvgdb::OpenVgdbClient;
use service::providers::retroachievements::RetroAchievementsClient;
use service::providers::screenscraper::ScreenScraperClient;
use service::providers::steamgriddb::SteamGridDbClient;
use service::providers::thegamesdb::TheGamesDbClient;
use service::providers::{MetadataProvider, ProviderRegistry};
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio_cron_scheduler::{Job, JobScheduler};
#[doc(hidden)]
pub use util::http::ReverProxyExtractor;
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

	let governor_conf = public_api_governor_config();

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

	spawn_db_pool_metrics(conn.clone());

	let sched = JobScheduler::new().await?;

	// Install the API key pepper before the server accepts requests. Missing or
	// malformed pepper is a loud startup panic.
	let pepper_raw = env::var("API_KEY_PEPPER").expect(
		"API_KEY_PEPPER environment variable is required (e.g. output of `openssl rand -hex 32`)",
	);
	service::db::user::init_pepper(&pepper_raw).unwrap_or_else(|e| panic!("API_KEY_PEPPER: {e}"));

	let igdb_http_client = Client::builder().cookie_store(true).build()?;

	let sgdb_http_client = Client::builder().cookie_store(false).build()?;

	let ss_http_client = Client::builder().cookie_store(false).build()?;

	let mg_http_client = Client::builder().cookie_store(false).build()?;

	let lb_http_client = Client::builder().cookie_store(false).build()?;

	let ovgdb_http_client = Client::builder().cookie_store(false).build()?;

	let ra_http_client = Client::builder().cookie_store(false).build()?;

	let er_http_client = Client::builder().cookie_store(false).build()?;

	let hasheous_http_client = Client::builder().cookie_store(false).build()?;

	let tgdb_http_client = Client::builder().cookie_store(false).build()?;

	// DAT downloads use a cookieless client so hostile mirrors cannot set cookies that
	// would replay on subsequent requests to the same host.
	let dat_http_client = Client::builder().cookie_store(false).build()?;

	let redis_client = redis::Client::open(env::var("REDIS_URL")?)?;

	let redis_conn = redis_client.get_multiplexed_async_connection().await?;
	info!("Connected to Redis");

	let igdb_client_opt = build_igdb_client(igdb_http_client, redis_conn.clone());
	let sgdb_client_opt = build_sgdb_client(sgdb_http_client, redis_conn.clone());
	let ss_client_opt = build_screenscraper_client(ss_http_client, redis_conn.clone());
	let mg_client_opt = build_mobygames_client(mg_http_client, redis_conn.clone());
	let lb_client_opt = build_launchbox_client(lb_http_client, redis_conn.clone(), conn.clone());
	let ovgdb_client_opt =
		build_openvgdb_client(ovgdb_http_client, redis_conn.clone(), conn.clone());
	let ra_client_opt =
		build_retroachievements_client(ra_http_client, redis_conn.clone(), conn.clone());
	let er_client_opt = build_emuready_client(er_http_client, redis_conn.clone());
	let hasheous_client_opt = build_hasheous_client(hasheous_http_client, redis_conn.clone());
	let tgdb_client_opt =
		build_thegamesdb_client(tgdb_http_client, redis_conn.clone(), conn.clone());

	let prometheus = PrometheusMetricsBuilder::new("api")
		.mask_unmatched_patterns("UNKNOWN")
		.build()
		.map_err(|e| anyhow!(e))?;

	service::metrics::init(&prometheus.registry)?;

	let metrics_registry = Data::new(prometheus.registry.clone());

	let conn_arc = Arc::new(conn);
	let dat_http_client_arc = Arc::new(dat_http_client);

	// Providers whose env vars are absent are skipped with a warn rather than
	// a panic so self-hosters can run with any subset enabled.
	let mut providers: ProviderRegistry = Vec::new();
	if let Some(c) = igdb_client_opt.clone() {
		providers.push(c as Arc<dyn MetadataProvider>);
	}
	if let Some(c) = sgdb_client_opt.clone() {
		providers.push(c as Arc<dyn MetadataProvider>);
	}
	if let Some(c) = ss_client_opt.clone() {
		providers.push(c as Arc<dyn MetadataProvider>);
	}
	if let Some(c) = mg_client_opt.clone() {
		providers.push(c as Arc<dyn MetadataProvider>);
	}
	if let Some(c) = lb_client_opt.clone() {
		providers.push(c as Arc<dyn MetadataProvider>);
	}
	if let Some(c) = ovgdb_client_opt.clone() {
		providers.push(c as Arc<dyn MetadataProvider>);
	}
	if let Some(c) = ra_client_opt.clone() {
		providers.push(c as Arc<dyn MetadataProvider>);
	}
	if let Some(c) = er_client_opt.clone() {
		providers.push(c as Arc<dyn MetadataProvider>);
	}
	if let Some(c) = hasheous_client_opt.clone() {
		providers.push(c as Arc<dyn MetadataProvider>);
	}
	if let Some(c) = tgdb_client_opt.clone() {
		providers.push(c as Arc<dyn MetadataProvider>);
	}
	if providers.is_empty() {
		warn!("No metadata providers configured. Background match cron will be a no-op.");
	}
	let providers_arc = Arc::new(providers);

	let redis_client_data = Data::new(redis_client);
	let redis_conn_for_cron = redis_conn.clone();
	let redis_conn_for_dat_cron = redis_conn.clone();
	let redis_conn_for_init = redis_conn.clone();
	let redis_conn_for_mcp = redis_conn.clone();
	let redis_conn_data = Data::new(redis_conn);
	let conn_data = Data::from(conn_arc.clone());
	let igdb_data = igdb_client_opt.clone().map(Data::from);
	let igdb_enabled = igdb_data.is_some();
	let sgdb_data = sgdb_client_opt.clone().map(Data::from);
	let sgdb_enabled = sgdb_data.is_some();
	let ss_data = ss_client_opt.clone().map(Data::from);
	let ss_enabled = ss_data.is_some();
	let mg_data = mg_client_opt.clone().map(Data::from);
	let mg_enabled = mg_data.is_some();
	let lb_enabled = lb_client_opt.is_some();
	let ovgdb_enabled = ovgdb_client_opt.is_some();
	let ra_enabled = ra_client_opt.is_some();

	let mcp_enabled = env::var("MCP_ENABLED")
		.unwrap_or_else(|_| "true".to_string())
		.eq_ignore_ascii_case("true");
	// Built once and cloned into each worker so the MCP session manager is shared.
	let mcp_service =
		mcp_enabled.then(|| mcp::build_mcp_service(conn_arc.clone(), redis_conn_for_mcp));
	let mcp_public_url = env::var("MCP_PUBLIC_URL")
		.ok()
		.filter(|s| !s.trim().is_empty());
	if mcp_enabled && mcp_public_url.is_none() {
		warn!(
			"MCP enabled but MCP_PUBLIC_URL is unset; the server card will omit the remote endpoint"
		);
	}
	let mcp_card =
		mcp_enabled.then(|| Data::new(McpCard(mcp::server_card_json(mcp_public_url.as_deref()))));

	// Resolved once at boot; the bare /api alias mirrors this version's routes
	// and header value for the lifetime of the process.
	let default_api_version = service::config::versions::resolve_default_version();
	info!("Default API version for the bare /api alias: {default_api_version}");

	let serv = HttpServer::new(move || {
		let mut app = App::new()
			.app_data(JsonConfig::default().limit(64 * 1024))
			.app_data(PayloadConfig::default().limit(256 * 1024))
			.app_data(conn_data.clone())
			.app_data(redis_client_data.clone())
			.app_data(redis_conn_data.clone());
		if let Some(d) = &igdb_data {
			app = app.app_data(d.clone());
		}
		if let Some(d) = &sgdb_data {
			app = app.app_data(d.clone());
		}
		if let Some(d) = &ss_data {
			app = app.app_data(d.clone());
		}
		if let Some(d) = &mg_data {
			app = app.app_data(d.clone());
		}
		let flags = PublicRouteFlags {
			igdb_enabled,
			sgdb_enabled,
			ss_enabled,
			mg_enabled,
			lb_enabled,
			ovgdb_enabled,
			ra_enabled,
		};

		// Explicit version scopes register before the bare /api alias because
		// actix matches overlapping prefixes by registration order and "/api"
		// is a prefix of "/api/v1".
		app = app.service(versioned_api_scope(
			"/api/v1",
			ApiVersion::V1,
			flags,
			&governor_conf,
			prometheus.clone(),
		));
		app = app.service(versioned_api_scope(
			"/api/v2",
			ApiVersion::V2,
			flags,
			&governor_conf,
			prometheus.clone(),
		));
		// The discovery doc registers before the bare /api alias: "/api" is a
		// prefix of "/api/versions", so the alias would otherwise swallow it.
		app = app
			.app_data(Data::new(ResolvedDefaultVersion(default_api_version)))
			.route("/api/versions", web::get().to(get_api_versions));
		app = app.service(versioned_api_scope(
			"/api",
			default_api_version,
			flags,
			&governor_conf,
			prometheus.clone(),
		));
		if let Some(svc) = &mcp_service {
			let mcp_cors = Cors::default()
				.allow_any_origin()
				.allowed_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
				.allowed_headers([
					header::CONTENT_TYPE,
					header::ACCEPT,
					header::HeaderName::from_static("mcp-session-id"),
					header::HeaderName::from_static("mcp-protocol-version"),
					header::HeaderName::from_static("last-event-id"),
				])
				.expose_headers([
					header::HeaderName::from_static("mcp-session-id"),
					header::HeaderName::from_static("mcp-protocol-version"),
				])
				.max_age(3600);
			app = app.service(
				scope("/mcp")
					.wrap(Governor::new(&governor_conf))
					.wrap(from_fn(http_request_metrics))
					.wrap(
						Logger::new("%{r}a %t \"%r\" %s %b \"%{User-Agent}i\" %T")
							.log_level(Level::Debug),
					)
					.wrap(
						DefaultHeaders::new()
							.add((
								"Strict-Transport-Security",
								"max-age=31536000; includeSubDomains",
							))
							.add(("X-Content-Type-Options", "nosniff")),
					)
					.wrap(mcp_cors)
					.service(svc.clone().scope()),
			);
		}
		if let Some(card) = &mcp_card {
			app = app
				.app_data(card.clone())
				.route(
					"/.well-known/mcp-server-card",
					web::get().to(serve_mcp_card),
				)
				.route("/.well-known/mcp.json", web::get().to(serve_mcp_card));
		}
		app.service(SwaggerUi::new("/swagger-ui/{_:.*}").urls(vec![
			(
				Url::with_primary("playmatch API v2", "/api-docs/v2/openapi.json", true),
				create_openapi_v2(),
			),
			(
				Url::new("playmatch API v1", "/api-docs/v1/openapi.json"),
				create_openapi_v1(),
			),
			(
				Url::new("playmatch API (alias)", "/api-docs/openapi.json"),
				create_openapi_v1(),
			),
		]))
	})
	.bind(format!("0.0.0.0:{port}"))?
	.shutdown_timeout(15)
	.workers(worker_amount)
	.run();

	// Without this lock the noon-UTC cron and the INITIAL_DATA_INIT spawn can
	// race on a boot near noon, doubling outbound quota burn and racing into
	// the signature_metadata_mapping unique indexes.
	let maintenance_lock: Arc<tokio::sync::Mutex<()>> = Arc::new(tokio::sync::Mutex::new(()));

	let conn = conn_arc.clone();
	let dat_client = dat_http_client_arc.clone();
	let providers_for_cron = providers_arc.clone();
	let lb_for_cron = lb_client_opt.clone();
	let ovgdb_for_cron = ovgdb_client_opt.clone();
	let ra_for_cron = ra_client_opt.clone();
	let maintenance_lock_cron = maintenance_lock.clone();
	sched
		.add(Job::new_async("0 0 12 * * *", move |_, _| {
			let conn = conn.clone();
			let dat_client = dat_client.clone();
			let dat_redis = redis_conn_for_dat_cron.clone();
			let providers = providers_for_cron.clone();
			let lb = lb_for_cron.clone();
			let ovgdb = ovgdb_for_cron.clone();
			let ra = ra_for_cron.clone();
			let lock = maintenance_lock_cron.clone();
			Box::pin(async move {
				let _guard = lock.lock().await;
				wrap_download_and_parse_dats(dat_client, conn.clone(), dat_redis, false).await;
				wrap_launchbox_import(lb).await;
				wrap_openvgdb_import(ovgdb).await;
				wrap_retroachievements_import(ra).await;
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
	let lb_for_init = lb_client_opt.clone();
	let ovgdb_for_init = ovgdb_client_opt.clone();
	let ra_for_init = ra_client_opt.clone();

	let initial_data_init = env::var("INITIAL_DATA_INIT")
		.unwrap_or("true".to_string())
		.to_lowercase()
		== "true";

	let force_initial_data_init = env::var("FORCE_INITIAL_DATA_INIT")
		.unwrap_or("false".to_string())
		.to_lowercase()
		== "true";

	if initial_data_init {
		let maintenance_lock_init = maintenance_lock.clone();
		tokio::spawn(async move {
			let _guard = maintenance_lock_init.lock().await;
			wrap_download_and_parse_dats(
				http_client,
				conn.clone(),
				redis_conn_for_init,
				force_initial_data_init,
			)
			.await;
			wrap_launchbox_import(lb_for_init).await;
			wrap_openvgdb_import(ovgdb_for_init).await;
			wrap_retroachievements_import(ra_for_init).await;
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
	if mcp_enabled {
		info!("MCP server mounted at /mcp on port {port}");
	} else {
		info!("MCP server disabled (set MCP_ENABLED=true to enable)");
	}
	tokio::try_join!(serv, metrics_serv)?;

	Ok(())
}

struct McpCard(String);

async fn serve_mcp_card(card: Data<McpCard>) -> HttpResponse {
	HttpResponse::Ok()
		.content_type("application/json")
		.insert_header(("Access-Control-Allow-Origin", "*"))
		.insert_header(("Access-Control-Allow-Methods", "GET"))
		.insert_header(("Access-Control-Allow-Headers", "Content-Type"))
		.insert_header(("Cache-Control", "public, max-age=3600"))
		.insert_header(("X-Content-Type-Options", "nosniff"))
		.body(card.0.clone())
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

/// Background task that samples the sqlx pool every 10 seconds and updates the
/// `api_db_pool_connections{state=...}` gauge. Lets dashboards alert on pool
/// saturation without external probing.
fn spawn_db_pool_metrics(conn: sea_orm::DatabaseConnection) {
	tokio::spawn(async move {
		let pool = conn.get_postgres_connection_pool().clone();
		let max = pool.options().get_max_connections() as i64;
		service::metrics::set_db_pool_connections("max", max);
		let mut ticker = tokio::time::interval(Duration::from_secs(10));
		ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
		loop {
			ticker.tick().await;
			let total = pool.size() as i64;
			let idle = pool.num_idle() as i64;
			let active = (total - idle).max(0);
			service::metrics::set_db_pool_connections("total", total);
			service::metrics::set_db_pool_connections("idle", idle);
			service::metrics::set_db_pool_connections("active", active);
		}
	});
}

/// Returns `None` when developer credentials are absent so self-hosters can
/// run without the ScreenScraper integration. User credentials are optional;
/// without them ScreenScraper heavily throttles and frequently rejects
/// requests, so warn loudly to set the operator's expectations.
fn build_screenscraper_client(
	http: Client,
	redis_conn: redis::aio::MultiplexedConnection,
) -> Option<Arc<ScreenScraperClient>> {
	let dev_id = match env::var("SCREENSCRAPER_DEV_ID") {
		Ok(v) if !v.trim().is_empty() => v,
		_ => {
			warn!("SCREENSCRAPER_DEV_ID not set, ScreenScraper provider disabled");
			return None;
		}
	};
	let dev_password = match env::var("SCREENSCRAPER_DEV_PASSWORD") {
		Ok(v) if !v.trim().is_empty() => v,
		_ => {
			warn!("SCREENSCRAPER_DEV_PASSWORD not set, ScreenScraper provider disabled");
			return None;
		}
	};
	let user_id = env::var("SCREENSCRAPER_USER_ID")
		.ok()
		.filter(|v| !v.trim().is_empty());
	let user_password = env::var("SCREENSCRAPER_USER_PASSWORD")
		.ok()
		.filter(|v| !v.trim().is_empty());
	let user = match (user_id, user_password) {
		(Some(id), Some(pw)) => Some((id, pw)),
		(None, None) => {
			warn!(
				"SCREENSCRAPER_USER_ID/SCREENSCRAPER_USER_PASSWORD not set; ScreenScraper will run anonymously and is heavily throttled"
			);
			None
		}
		_ => {
			warn!(
				"SCREENSCRAPER_USER_ID and SCREENSCRAPER_USER_PASSWORD must be set together; treating as anonymous"
			);
			None
		}
	};
	match ScreenScraperClient::new(dev_id, dev_password, user, http, redis_conn) {
		Ok(c) => {
			info!("ScreenScraper provider enabled");
			Some(Arc::new(c))
		}
		Err(e) => {
			warn!("ScreenScraper provider construction failed, disabled: {e}");
			None
		}
	}
}

/// Returns `None` when EMUREADY_ENABLED is unset or not "true". The EmuReady
/// API is open and unauthenticated for read endpoints; the flag exists so
/// existing deployments do not silently start hitting an external service on
/// next deploy. Matching only. No proxy routes are exposed.
fn build_emuready_client(
	http: Client,
	redis_conn: redis::aio::MultiplexedConnection,
) -> Option<Arc<EmuReadyClient>> {
	let enabled = env::var("EMUREADY_ENABLED")
		.unwrap_or_default()
		.eq_ignore_ascii_case("true");
	if !enabled {
		warn!("EMUREADY_ENABLED not set to true, EmuReady provider disabled");
		return None;
	}
	match EmuReadyClient::new(http, redis_conn) {
		Ok(c) => {
			info!("EmuReady provider enabled");
			Some(Arc::new(c))
		}
		Err(e) => {
			warn!("EmuReady provider construction failed, disabled: {e}");
			None
		}
	}
}

/// Returns `None` when HASHEOUS_ENABLED is unset or not "true". The Hasheous
/// hash-lookup endpoints are open and unauthenticated; the flag exists so
/// existing deployments do not silently start hitting an external service on
/// next deploy. Matching only. No proxy routes are exposed.
fn build_hasheous_client(
	http: Client,
	redis_conn: redis::aio::MultiplexedConnection,
) -> Option<Arc<HasheousClient>> {
	let enabled = env::var("HASHEOUS_ENABLED")
		.unwrap_or_default()
		.eq_ignore_ascii_case("true");
	if !enabled {
		warn!("HASHEOUS_ENABLED not set to true, Hasheous provider disabled");
		return None;
	}
	match HasheousClient::new(http, redis_conn) {
		Ok(c) => {
			info!("Hasheous provider enabled");
			Some(Arc::new(c))
		}
		Err(e) => {
			warn!("Hasheous provider construction failed, disabled: {e}");
			None
		}
	}
}

/// Returns `None` when TGDB_ENABLED is unset or not "true". TheGamesDB is
/// match-only; no proxy routes are exposed. The bootstrap dataset is seeded
/// via a migration so no daily import job runs. TGDB_API_KEY is optional;
/// without it the provider still runs but skips the search-on-miss API rung
/// and falls back to local-only matching.
fn build_thegamesdb_client(
	http: Client,
	redis_conn: redis::aio::MultiplexedConnection,
	db_conn: sea_orm::DbConn,
) -> Option<Arc<TheGamesDbClient>> {
	let enabled = env::var("TGDB_ENABLED")
		.unwrap_or_default()
		.eq_ignore_ascii_case("true");
	if !enabled {
		warn!("TGDB_ENABLED not set to true, TheGamesDB provider disabled");
		return None;
	}
	let api_key = env::var("TGDB_API_KEY")
		.ok()
		.filter(|v| !v.trim().is_empty());
	if api_key.is_none() {
		warn!(
			"TGDB_API_KEY not set; TheGamesDB will match locally against the seeded dataset only"
		);
	}
	match TheGamesDbClient::new(http, redis_conn, db_conn, api_key) {
		Ok(c) => {
			info!("TheGamesDB provider enabled");
			Some(Arc::new(c))
		}
		Err(e) => {
			warn!("TheGamesDB provider construction failed, disabled: {e}");
			None
		}
	}
}

/// Returns `None` when LAUNCHBOX_ENABLED is unset or not "true". The
/// LaunchBox provider downloads ~70 MB of XML once a day and persists ~700k
/// rows into Postgres, so it is opt-in to keep small deployments lean.
fn build_launchbox_client(
	http: Client,
	redis_conn: redis::aio::MultiplexedConnection,
	db_conn: sea_orm::DbConn,
) -> Option<Arc<LaunchBoxClient>> {
	let enabled = env::var("LAUNCHBOX_ENABLED")
		.unwrap_or_default()
		.eq_ignore_ascii_case("true");
	if !enabled {
		warn!("LAUNCHBOX_ENABLED not set to true, LaunchBox provider disabled");
		return None;
	}
	let metadata_url = env::var("LAUNCHBOX_METADATA_URL")
		.ok()
		.filter(|v| !v.trim().is_empty());
	match LaunchBoxClient::new(http, redis_conn, db_conn, metadata_url) {
		Ok(c) => {
			info!("LaunchBox provider enabled");
			Some(Arc::new(c))
		}
		Err(e) => {
			warn!("LaunchBox provider construction failed, disabled: {e}");
			None
		}
	}
}

/// Returns `None` when OPENVGDB_ENABLED is unset or not "true". The OpenVGDB
/// provider downloads a SQLite snapshot and persists its roms and releases
/// into Postgres, so it is opt-in to keep small deployments lean.
fn build_openvgdb_client(
	http: Client,
	redis_conn: redis::aio::MultiplexedConnection,
	db_conn: sea_orm::DbConn,
) -> Option<Arc<OpenVgdbClient>> {
	let enabled = env::var("OPENVGDB_ENABLED")
		.unwrap_or_default()
		.eq_ignore_ascii_case("true");
	if !enabled {
		warn!("OPENVGDB_ENABLED not set to true, OpenVGDB provider disabled");
		return None;
	}
	let metadata_url = env::var("OPENVGDB_METADATA_URL")
		.ok()
		.filter(|v| !v.trim().is_empty());
	match OpenVgdbClient::new(http, redis_conn, db_conn, metadata_url) {
		Ok(c) => {
			info!("OpenVGDB provider enabled");
			Some(Arc::new(c))
		}
		Err(e) => {
			warn!("OpenVGDB provider construction failed, disabled: {e}");
			None
		}
	}
}

/// Returns `None` when either credential is missing so self-hosters can run
/// without the RetroAchievements integration. The provider bulk-imports RA's
/// game and hash lists once per cycle; the API key + username pair is needed
/// to make the underlying `API_GetGameList.php` calls.
fn build_retroachievements_client(
	http: Client,
	redis_conn: redis::aio::MultiplexedConnection,
	db_conn: sea_orm::DbConn,
) -> Option<Arc<RetroAchievementsClient>> {
	let username = match env::var("RETROACHIEVEMENTS_USERNAME") {
		Ok(v) if !v.trim().is_empty() => v,
		_ => {
			warn!("RETROACHIEVEMENTS_USERNAME not set, RetroAchievements provider disabled");
			return None;
		}
	};
	let api_key = match env::var("RETROACHIEVEMENTS_API_KEY") {
		Ok(v) if !v.trim().is_empty() => v,
		_ => {
			warn!("RETROACHIEVEMENTS_API_KEY not set, RetroAchievements provider disabled");
			return None;
		}
	};
	match RetroAchievementsClient::new(username, api_key, http, redis_conn, db_conn) {
		Ok(c) => {
			info!("RetroAchievements provider enabled");
			Some(Arc::new(c))
		}
		Err(e) => {
			warn!("RetroAchievements provider construction failed, disabled: {e}");
			None
		}
	}
}

/// Returns `None` when the API key is absent so self-hosters can run without
/// the MobyGames integration. The MobyGames API requires a paid subscription;
/// the cheapest tier permits 1 request every 5 seconds.
fn build_mobygames_client(
	http: Client,
	redis_conn: redis::aio::MultiplexedConnection,
) -> Option<Arc<MobyGamesClient>> {
	let api_key = match env::var("MOBYGAMES_API_KEY") {
		Ok(v) if !v.trim().is_empty() => v,
		_ => {
			warn!("MOBYGAMES_API_KEY not set, MobyGames provider disabled");
			return None;
		}
	};
	match MobyGamesClient::new(api_key, http, redis_conn) {
		Ok(c) => {
			info!("MobyGames provider enabled");
			Some(Arc::new(c))
		}
		Err(e) => {
			warn!("MobyGames provider construction failed, disabled: {e}");
			None
		}
	}
}

/// Returns `None` when the API key is absent so self-hosters can run without
/// the SGDB integration.
fn build_sgdb_client(
	http: Client,
	redis_conn: redis::aio::MultiplexedConnection,
) -> Option<Arc<SteamGridDbClient>> {
	let bearer = match env::var("STEAMGRIDDB_API_KEY") {
		Ok(v) if !v.trim().is_empty() => v,
		_ => {
			warn!("STEAMGRIDDB_API_KEY not set, SteamGridDB provider disabled");
			return None;
		}
	};
	match SteamGridDbClient::new(bearer, http, redis_conn) {
		Ok(c) => {
			info!("SteamGridDB provider enabled");
			Some(Arc::new(c))
		}
		Err(e) => {
			warn!("SteamGridDB provider construction failed, disabled: {e}");
			None
		}
	}
}

/// Returns `None` when credentials are absent so self-hosters can run with
/// any subset of providers enabled rather than panicking at boot.
fn build_igdb_client(
	http: Client,
	redis_conn: redis::aio::MultiplexedConnection,
) -> Option<Arc<IgdbClient>> {
	let client_id = match env::var("IGDB_CLIENT_ID") {
		Ok(v) if !v.trim().is_empty() => v,
		_ => {
			warn!("IGDB_CLIENT_ID not set, IGDB provider disabled");
			return None;
		}
	};
	let client_secret = match env::var("IGDB_CLIENT_SECRET") {
		Ok(v) if !v.trim().is_empty() => v,
		_ => {
			warn!("IGDB_CLIENT_SECRET not set, IGDB provider disabled");
			return None;
		}
	};
	match IgdbClient::new(client_id, client_secret, http, redis_conn) {
		Ok(c) => {
			info!("IGDB provider enabled");
			Some(Arc::new(c))
		}
		Err(e) => {
			warn!("IGDB provider construction failed, disabled: {e}");
			None
		}
	}
}

/// Builds the rate-limiter config used by the public API scopes. Exposed for
/// integration tests so they can construct the version scopes exactly as the
/// running server does without naming internal extractor types.
#[doc(hidden)]
pub fn public_api_governor_config()
-> GovernorConfig<ReverProxyExtractor, StateInformationMiddleware> {
	GovernorConfigBuilder::default()
		.use_headers()
		.milliseconds_per_request(250)
		.key_extractor(ReverProxyExtractor)
		.burst_size(20)
		.finish()
		.expect("governor config is valid by construction")
}

/// Builds a Prometheus middleware instance for the public API scopes. Exposed
/// for integration tests; each call owns a fresh registry so tests do not
/// collide on global metric registration.
#[doc(hidden)]
pub fn test_prometheus_metrics() -> actix_web_prom::PrometheusMetrics {
	PrometheusMetricsBuilder::new("api")
		.mask_unmatched_patterns("UNKNOWN")
		.build()
		.expect("prometheus middleware builds")
}

#[doc(hidden)]
#[derive(Clone, Copy, Default)]
pub struct PublicRouteFlags {
	pub igdb_enabled: bool,
	pub sgdb_enabled: bool,
	pub ss_enabled: bool,
	pub mg_enabled: bool,
	pub lb_enabled: bool,
	pub ovgdb_enabled: bool,
	pub ra_enabled: bool,
}

/// Builds one path-versioned API scope reproducing the exact shared middleware
/// stack (Governor, request metrics, Logger, security headers, permissive CORS,
/// Prometheus, Compress) plus the always-on `Playmatch-Api-Version` header and
/// the nested authenticated sub-scope. The bare `/api` alias passes the
/// resolved default version so it reports that version's header value.
///
/// `version` selects which route set is mounted: V1 carries the full current
/// surface, while later versions mount only their own handlers (none yet) so v2
/// does not silently inherit v1 routes.
#[doc(hidden)]
pub fn versioned_api_scope(
	path: &str,
	version: ApiVersion,
	flags: PublicRouteFlags,
	governor_conf: &GovernorConfig<ReverProxyExtractor, StateInformationMiddleware>,
	prometheus: actix_web_prom::PrometheusMetrics,
) -> impl HttpServiceFactory + use<> {
	scope(path)
		.wrap(Governor::new(governor_conf))
		// Wraps outside the rate limiter so a v2 429 is rewritten into the JSON
		// envelope too; version-aware, so v1 passes through as plain text.
		.wrap(from_fn(v2_error_envelope(version)))
		.wrap(from_fn(http_request_metrics))
		.wrap(from_fn(version_lifecycle_headers(version)))
		.wrap(Logger::new("%{r}a %t \"%r\" %s %b \"%{User-Agent}i\" %T").log_level(Level::Debug))
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
		.wrap(DefaultHeaders::new().add(("Playmatch-Api-Version", version.as_str())))
		.wrap(Cors::permissive())
		.wrap(prometheus)
		.wrap(Compress::default())
		.configure(move |cfg| configure_version_routes(cfg, version, flags))
}

fn configure_version_routes(cfg: &mut ServiceConfig, version: ApiVersion, flags: PublicRouteFlags) {
	match version {
		ApiVersion::V1 => {
			configure_public_api_routes(
				cfg,
				flags.igdb_enabled,
				flags.sgdb_enabled,
				flags.ss_enabled,
				flags.mg_enabled,
				flags.lb_enabled,
				flags.ovgdb_enabled,
				flags.ra_enabled,
			);
			cfg.service(
				scope("")
					.wrap(
						DefaultHeaders::new()
							.add(("Cache-Control", "no-store"))
							.add(("Vary", "Authorization")),
					)
					.configure(configure_authenticated_api_routes),
			);
		}
		ApiVersion::V2 => {
			configure_public_api_routes_v2(cfg, flags);
		}
	}
}

/// The 256 KiB JsonConfig for the v2 bulk endpoints, with an error handler that
/// turns a malformed, mistyped, or oversized JSON body into the v2 `V2ErrorBody`
/// envelope. Scoped to the v2 surface only; v1 keeps the global plain-text
/// JsonConfig. Every `JsonPayloadError` (deserialize failure, wrong content type,
/// or overflow past the cap) maps to a single `malformed_body` 400 so a bad bulk
/// body never reaches the client as actix's default plain-text error.
fn v2_json_config() -> JsonConfig {
	JsonConfig::default()
		.limit(256 * 1024)
		.error_handler(|err, _req| {
			let response = crate::routes::v2::error::v2_malformed_body(err.to_string());
			actix_web::error::InternalError::from_response(err, response).into()
		})
}

/// v2 public surface. Self-contained: it carries its own paginated reference
/// lists plus singular reads that reuse the same service fns as v1, and it
/// mounts the version-agnostic shared routes and provider proxies so the v2
/// scope answers every resource v1 does without registering two services at the
/// same literal path. `flags` gate the provider proxies exactly as in v1.
fn configure_public_api_routes_v2(cfg: &mut ServiceConfig, flags: PublicRouteFlags) {
	// Everything mounts inside ONE empty-prefix scope. An actix `scope("")` matches
	// every path, so a sibling empty scope registered ahead of other services
	// swallows their requests and 404s them; a single scope avoids that. It also
	// carries the larger 256 KiB JsonConfig so a full bulk batch (up to 100 items
	// with hashes) clears the global 64 KiB body limit, which stays untouched
	// elsewhere. Each bulk batch still counts as one request against the version
	// scope's shared Governor.
	//
	// Literal paths (`/games/bulk`, `/games/search`, `/games/by-name`, the
	// `/games/{id}/...` sub-resources, `/companies/search`, `/platforms/search`,
	// `/signature-groups/search`, `/dat-files/by-hash`, `/dat-files/bulk`)
	// register before their `/{id}` dynamic peers so the dynamic segment cannot
	// swallow them.
	cfg.service(
		scope("")
			.app_data(v2_json_config())
			.service(bulk_games_by_id_v2)
			.service(bulk_platforms_by_id_v2)
			.service(bulk_companies_by_id_v2)
			.service(bulk_signature_groups_by_id_v2)
			.service(bulk_dat_files_by_id_v2)
			.service(bulk_game_files_by_id_v2)
			.service(identify_bulk_ids_v2)
			.service(identify_bulk_relations_v2)
			.service(get_stats_v2)
			.service(list_companies_v2)
			.service(search_companies_v2)
			.service(get_company_by_id_v2)
			.service(list_platforms_v2)
			.service(get_platform_stats_v2)
			.service(search_platforms_v2)
			.service(get_platform_by_id_v2)
			.service(list_signature_groups_v2)
			.service(list_signature_group_dat_files_v2)
			.service(list_signature_group_games_v2)
			.service(search_signature_groups_v2)
			.service(get_signature_group_by_id_v2)
			.service(list_games_v2)
			.service(search_games_v2)
			.service(get_games_by_name_v2)
			.service(get_game_file_presence_v2)
			.service(get_game_file_by_id_v2)
			.service(get_game_with_relations_by_id_v2)
			.service(list_game_files_v2)
			.service(list_game_mappings_v2)
			.service(list_game_clones_v2)
			.service(list_game_dat_files_v2)
			.service(get_game_by_id_v2)
			.service(list_dat_files_v2)
			.service(list_dat_files_by_hash_v2)
			.service(list_dat_file_games_v2)
			.service(get_dat_file_import_v2)
			.service(list_dat_file_imports_v2)
			.service(get_dat_file_by_id_v2)
			.configure(configure_shared_public_routes)
			.configure(move |c| configure_provider_proxy_routes(c, flags))
			.service(
				scope("")
					.wrap(
						DefaultHeaders::new()
							.add(("Cache-Control", "no-store"))
							.add(("Vary", "Authorization")),
					)
					.configure(configure_authenticated_api_routes_v2),
			),
	);
}

/// v2 authenticated surface. Reuses the v1 suggestion and manual match handlers,
/// which enforce their own permission level on the request and already serialize
/// camelCase, and adds the v2 keyset suggestion list. The two user endpoints that
/// take a request body are mounted as forked handlers reading the camelCase
/// request bodies; `get_user` carries no body or query and reuses the v1 handler.
/// `get_user_by_discord_id_v2` is forked so its `discordId` query param is
/// camelCase. `get_all_suggestions` is intentionally omitted because
/// `list_suggestions_v2` answers `/suggestion` for this version.
fn configure_authenticated_api_routes_v2(cfg: &mut ServiceConfig) {
	cfg.service(list_suggestions_v2)
		.service(get_suggestion_by_id)
		.service(create_game_suggestion)
		.service(create_company_suggestion)
		.service(create_platform_suggestion)
		.service(approve_suggestion)
		.service(delete_suggestion)
		.service(manually_match_game)
		.service(manually_match_platform)
		.service(manually_match_company)
		.service(create_or_get_by_discord_id_v2)
		.service(get_user_by_discord_id_v2)
		.service(get_user)
		.service(update_user_permission_level_v2);
}

#[allow(clippy::too_many_arguments)]
fn configure_public_api_routes(
	cfg: &mut ServiceConfig,
	igdb_enabled: bool,
	sgdb_enabled: bool,
	ss_enabled: bool,
	mg_enabled: bool,
	lb_enabled: bool,
	ovgdb_enabled: bool,
	ra_enabled: bool,
) {
	configure_provider_routes(
		cfg,
		PublicRouteFlags {
			igdb_enabled,
			sgdb_enabled,
			ss_enabled,
			mg_enabled,
			lb_enabled,
			ovgdb_enabled,
			ra_enabled,
		},
	);
}

/// Registers the full v1 public surface: health and readiness probes, the
/// external suggestion intake, the company, platform, signature group, identify,
/// and playmatch game reads, and the optional provider proxy routes gated by
/// `flags`. The registration order matches the historical v1 order so v1
/// reproduces its existing route set byte for byte after the refactor.
///
/// v2 does not call this because it carries its own paginated equivalents for
/// the company, platform, signature group, game read, and search resources at
/// the same literal paths; v2 instead mounts only the collision-free shared
/// routes via [`configure_shared_public_routes`] and the provider proxies via
/// [`configure_provider_proxy_routes`].
fn configure_provider_routes(cfg: &mut ServiceConfig, flags: PublicRouteFlags) {
	cfg.service(health)
		.service(ready)
		.service(submit_external_game_suggestion)
		.service(get_all_companies)
		.service(get_company_by_id)
		.service(get_all_platforms)
		.service(get_platform_by_id)
		.service(get_all_signature_groups)
		.service(get_signature_group_by_id)
		.service(identify_game_with_metadata_ids)
		.service(identify_game_and_relations)
		.service(get_playmatch_game_by_id)
		.service(get_playmatch_game_with_relations_by_id)
		.service(search_games)
		.service(get_game_file_history_by_id);
	configure_provider_proxy_routes(cfg, flags);
}

/// Registers the public routes the v2 scope reuses from the shared surface.
/// `/identify/ids`, the relations identify, and the game-file history are all
/// mounted as v2 forks: `identify_game_with_metadata_ids_v2` answers the v2
/// `{code, message}` envelope on a malformed query (matching the relations fork)
/// instead of v1's plain-text body, and the relations/history forks emit the
/// camelCase wrapper types over the snake_case playmatch objects. Excludes the
/// company, platform, signature group, `/game/{id}` reads, and `/games/search`
/// handlers whose paths v2 already owns through its own handlers.
fn configure_shared_public_routes(cfg: &mut ServiceConfig) {
	cfg.service(health)
		.service(ready)
		.service(submit_external_game_suggestion)
		.service(identify_game_with_metadata_ids_v2)
		.service(identify_game_and_relations_v2)
		.service(get_game_file_history_by_id_v2);
}

/// Registers the optional external provider proxy routes gated by `flags`. These
/// proxies are version-agnostic and have no v2-specific equivalents, so both v1
/// and v2 mount the identical set.
fn configure_provider_proxy_routes(cfg: &mut ServiceConfig, flags: PublicRouteFlags) {
	if flags.igdb_enabled {
		configure_igdb_routes(cfg);
	}
	if flags.sgdb_enabled {
		configure_sgdb_routes(cfg);
	}
	if flags.ss_enabled {
		configure_screenscraper_routes(cfg);
	}
	if flags.mg_enabled {
		configure_mobygames_routes(cfg);
	}
	if flags.lb_enabled {
		configure_launchbox_routes(cfg);
	}
	if flags.ovgdb_enabled {
		configure_openvgdb_routes(cfg);
	}
	if flags.ra_enabled {
		configure_retroachievements_routes(cfg);
	}
}

fn configure_retroachievements_routes(cfg: &mut ServiceConfig) {
	cfg.service(list_ra_systems)
		.service(get_ra_game_by_id)
		.service(get_ra_game_by_hash)
		.service(search_ra_games);
}

fn configure_launchbox_routes(cfg: &mut ServiceConfig) {
	cfg.service(list_lb_platforms)
		.service(get_lb_game_by_id)
		.service(search_lb_games)
		.service(get_lb_game_alternate_names)
		.service(get_lb_game_images);
}

fn configure_openvgdb_routes(cfg: &mut ServiceConfig) {
	cfg.service(get_ovgdb_release_by_id)
		.service(get_ovgdb_rom_by_hash);
}

fn configure_mobygames_routes(cfg: &mut ServiceConfig) {
	cfg.service(list_mg_platforms)
		.service(list_mg_genres)
		.service(get_mg_game_by_id)
		.service(search_mg_games)
		.service(get_mg_game_covers)
		.service(get_mg_game_screenshots);
}

fn configure_screenscraper_routes(cfg: &mut ServiceConfig) {
	cfg.service(list_ss_systems)
		.service(get_ss_game_by_id)
		.service(search_ss_games)
		.service(get_ss_game_by_rom_name);
}

fn configure_sgdb_routes(cfg: &mut ServiceConfig) {
	cfg.service(get_sgdb_game_by_id)
		.service(get_sgdb_game_by_platform)
		.service(search_sgdb_games)
		.service(get_sgdb_grids_by_game)
		.service(get_sgdb_grids_by_platform)
		.service(get_sgdb_heroes_by_game)
		.service(get_sgdb_heroes_by_platform)
		.service(get_sgdb_logos_by_game)
		.service(get_sgdb_logos_by_platform)
		.service(get_sgdb_icons_by_game)
		.service(get_sgdb_icons_by_platform);
}

fn configure_igdb_routes(cfg: &mut ServiceConfig) {
	cfg.service(get_igdb_game_by_id)
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

#[cfg(test)]
mod tests {
	use super::{McpCard, serve_mcp_card};
	use actix_web::web::Data;
	use actix_web::{App, test, web};

	#[actix_web::test]
	async fn well_known_card_routes_serve_json() {
		let card = Data::new(McpCard(mcp::server_card_json(Some("https://example.test"))));
		let app = test::init_service(
			App::new()
				.app_data(card.clone())
				.route(
					"/.well-known/mcp-server-card",
					web::get().to(serve_mcp_card),
				)
				.route("/.well-known/mcp.json", web::get().to(serve_mcp_card)),
		)
		.await;

		for path in ["/.well-known/mcp.json", "/.well-known/mcp-server-card"] {
			let req = test::TestRequest::get().uri(path).to_request();
			let resp = test::call_service(&app, req).await;
			assert!(resp.status().is_success(), "{path} should return 200");
			assert_eq!(
				resp.headers().get("content-type").unwrap(),
				"application/json"
			);
			assert_eq!(
				resp.headers().get("access-control-allow-origin").unwrap(),
				"*"
			);

			let body = test::read_body(resp).await;
			let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
			assert_eq!(json["name"], "io.github.retrorealm/playmatch");
			assert_eq!(json["remotes"][0]["url"], "https://example.test/mcp");
		}
	}
}
