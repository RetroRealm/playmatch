use prometheus::{
	HistogramOpts, HistogramVec, IntCounterVec, IntGauge, IntGaugeVec, Opts, Registry,
};
use std::sync::OnceLock;

static CACHE_EVENTS: OnceLock<IntCounterVec> = OnceLock::new();
static CACHE_L1_SIZE: OnceLock<IntGaugeVec> = OnceLock::new();
static IDENTIFY_ATTEMPTS: OnceLock<IntCounterVec> = OnceLock::new();
static SERVICE_ERRORS: OnceLock<IntCounterVec> = OnceLock::new();
static METADATA_AUTO_MATCHES: OnceLock<IntCounterVec> = OnceLock::new();
static MATCH_RUNG_OUTCOMES: OnceLock<IntCounterVec> = OnceLock::new();
static METADATA_TOKEN_REFRESHES: OnceLock<IntCounterVec> = OnceLock::new();
static BACKGROUND_JOB_RUNS: OnceLock<IntCounterVec> = OnceLock::new();
static BACKGROUND_JOB_DURATION: OnceLock<HistogramVec> = OnceLock::new();
static BACKGROUND_JOB_LAST_SUCCESS: OnceLock<IntGaugeVec> = OnceLock::new();
static METADATA_REQUESTS: OnceLock<IntCounterVec> = OnceLock::new();
static METADATA_REQUEST_DURATION: OnceLock<HistogramVec> = OnceLock::new();
static DAT_INGESTION_FILES: OnceLock<IntCounterVec> = OnceLock::new();
static CLONE_OF_RESOLUTIONS: OnceLock<IntCounterVec> = OnceLock::new();
static USER_ACTIONS: OnceLock<IntCounterVec> = OnceLock::new();
static USER_AGENTS: OnceLock<IntCounterVec> = OnceLock::new();
static LAUNCHBOX_IMPORT_RECORDS: OnceLock<IntCounterVec> = OnceLock::new();
static OPENVGDB_IMPORT_RECORDS: OnceLock<IntCounterVec> = OnceLock::new();
static RETROACHIEVEMENTS_IMPORT_RECORDS: OnceLock<IntCounterVec> = OnceLock::new();
static SCREENSCRAPER_QUOTA_EXHAUSTION: OnceLock<IntCounterVec> = OnceLock::new();
static METADATA_REQUEST_INFLIGHT: OnceLock<IntGaugeVec> = OnceLock::new();
static METADATA_REQUEST_ATTEMPTS: OnceLock<IntCounterVec> = OnceLock::new();
static PROVIDER_CONCURRENCY_CONFIGURED: OnceLock<IntGaugeVec> = OnceLock::new();
static CROSS_MATCH_ATTEMPTS: OnceLock<IntCounterVec> = OnceLock::new();
static THEGAMESDB_API_CALLS: OnceLock<IntCounterVec> = OnceLock::new();
static THEGAMESDB_CYCLE_CALLS: OnceLock<IntGauge> = OnceLock::new();
static THEGAMESDB_REMAINING_ALLOWANCE: OnceLock<IntGauge> = OnceLock::new();
static EXTERNAL_SUGGESTION_QUEUE_DEPTH: OnceLock<IntGauge> = OnceLock::new();
static HTTP_RATE_LIMIT_REJECTED: OnceLock<IntCounterVec> = OnceLock::new();
static HTTP_REQUESTS_INFLIGHT: OnceLock<IntGaugeVec> = OnceLock::new();
static DB_POOL_CONNECTIONS: OnceLock<IntGaugeVec> = OnceLock::new();

pub fn init(registry: &Registry) -> anyhow::Result<()> {
	let cache_events = IntCounterVec::new(
		Opts::new(
			"api_cache_events_total",
			"Cache hits and misses, labelled by subsystem and lookup kind",
		),
		&["cache", "lookup", "result"],
	)?;
	registry.register(Box::new(cache_events.clone()))?;
	CACHE_EVENTS
		.set(cache_events)
		.map_err(|_| anyhow::anyhow!("cache metrics already initialised"))?;

	let cache_l1_size = IntGaugeVec::new(
		Opts::new(
			"api_cache_l1_entries",
			"Current entry count of each in-process L1 (moka) cache, per lookup kind",
		),
		&["lookup"],
	)?;
	registry.register(Box::new(cache_l1_size.clone()))?;
	CACHE_L1_SIZE
		.set(cache_l1_size)
		.map_err(|_| anyhow::anyhow!("cache l1 size metrics already initialised"))?;

	let identify_attempts = IntCounterVec::new(
		Opts::new(
			"api_identify_match_attempts_total",
			"Identify pipeline attempts per hash type and outcome",
		),
		&["hash_type", "result"],
	)?;
	registry.register(Box::new(identify_attempts.clone()))?;
	IDENTIFY_ATTEMPTS
		.set(identify_attempts)
		.map_err(|_| anyhow::anyhow!("identify metrics already initialised"))?;

	let service_errors = IntCounterVec::new(
		Opts::new(
			"api_service_errors_total",
			"Errors returned to clients, labelled by collapsed variant",
		),
		&["variant"],
	)?;
	registry.register(Box::new(service_errors.clone()))?;
	SERVICE_ERRORS
		.set(service_errors)
		.map_err(|_| anyhow::anyhow!("service error metrics already initialised"))?;

	let metadata_auto_matches = IntCounterVec::new(
		Opts::new(
			"api_metadata_auto_match_total",
			"Automatic metadata-provider match outcomes per provider, entity type, result and reason",
		),
		&["provider", "entity_type", "result", "reason"],
	)?;
	registry.register(Box::new(metadata_auto_matches.clone()))?;
	METADATA_AUTO_MATCHES
		.set(metadata_auto_matches)
		.map_err(|_| anyhow::anyhow!("metadata auto match metrics already initialised"))?;

	let match_rung_outcomes = IntCounterVec::new(
		Opts::new(
			"api_match_rung_total",
			"Per-rung match attempt outcomes for each provider's matching ladder",
		),
		&["provider", "rung", "outcome"],
	)?;
	registry.register(Box::new(match_rung_outcomes.clone()))?;
	MATCH_RUNG_OUTCOMES
		.set(match_rung_outcomes)
		.map_err(|_| anyhow::anyhow!("match rung outcome metrics already initialised"))?;

	let metadata_token_refreshes = IntCounterVec::new(
		Opts::new(
			"api_metadata_token_refresh_total",
			"OAuth2 token refresh attempts by provider, trigger and result",
		),
		&["provider", "trigger", "result"],
	)?;
	registry.register(Box::new(metadata_token_refreshes.clone()))?;
	METADATA_TOKEN_REFRESHES
		.set(metadata_token_refreshes)
		.map_err(|_| anyhow::anyhow!("metadata token refresh metrics already initialised"))?;

	let background_job_runs = IntCounterVec::new(
		Opts::new(
			"api_background_job_total",
			"Background job completions by job and result",
		),
		&["job", "result"],
	)?;
	registry.register(Box::new(background_job_runs.clone()))?;
	BACKGROUND_JOB_RUNS
		.set(background_job_runs)
		.map_err(|_| anyhow::anyhow!("background job run metrics already initialised"))?;

	let background_job_duration = HistogramVec::new(
		HistogramOpts::new(
			"api_background_job_duration_seconds",
			"Background job runtime in seconds",
		)
		.buckets(vec![
			1.0, 10.0, 30.0, 60.0, 300.0, 900.0, 1800.0, 3600.0, 7200.0, 14400.0,
		]),
		&["job"],
	)?;
	registry.register(Box::new(background_job_duration.clone()))?;
	BACKGROUND_JOB_DURATION
		.set(background_job_duration)
		.map_err(|_| anyhow::anyhow!("background job duration metrics already initialised"))?;

	let background_job_last_success = IntGaugeVec::new(
		Opts::new(
			"api_background_job_last_success_unixtime",
			"Unix timestamp of the last successful completion of each background job",
		),
		&["job"],
	)?;
	registry.register(Box::new(background_job_last_success.clone()))?;
	BACKGROUND_JOB_LAST_SUCCESS
		.set(background_job_last_success)
		.map_err(|_| anyhow::anyhow!("background job last success metrics already initialised"))?;

	let metadata_requests = IntCounterVec::new(
		Opts::new(
			"api_metadata_request_total",
			"Metadata-provider outbound requests by provider, endpoint, HTTP status class and numeric status code",
		),
		&["provider", "endpoint", "status_class", "status_code"],
	)?;
	registry.register(Box::new(metadata_requests.clone()))?;
	METADATA_REQUESTS
		.set(metadata_requests)
		.map_err(|_| anyhow::anyhow!("metadata request metrics already initialised"))?;

	let metadata_request_duration = HistogramVec::new(
		HistogramOpts::new(
			"api_metadata_request_duration_seconds",
			"Metadata-provider outbound request duration in seconds, by provider, endpoint, HTTP status class and numeric status code",
		)
		.buckets(vec![
			0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0,
		]),
		&["provider", "endpoint", "status_class", "status_code"],
	)?;
	registry.register(Box::new(metadata_request_duration.clone()))?;
	METADATA_REQUEST_DURATION
		.set(metadata_request_duration)
		.map_err(|_| anyhow::anyhow!("metadata request duration metrics already initialised"))?;

	let dat_ingestion_files = IntCounterVec::new(
		Opts::new(
			"api_dat_ingestion_files_total",
			"DAT files processed during ingestion by source and outcome",
		),
		&["source", "outcome"],
	)?;
	registry.register(Box::new(dat_ingestion_files.clone()))?;
	DAT_INGESTION_FILES
		.set(dat_ingestion_files)
		.map_err(|_| anyhow::anyhow!("dat ingestion metrics already initialised"))?;

	let clone_of_resolutions = IntCounterVec::new(
		Opts::new(
			"api_clone_of_resolutions_total",
			"Clone-of relationship resolution outcomes",
		),
		&["result"],
	)?;
	registry.register(Box::new(clone_of_resolutions.clone()))?;
	CLONE_OF_RESOLUTIONS
		.set(clone_of_resolutions)
		.map_err(|_| anyhow::anyhow!("clone-of metrics already initialised"))?;

	let user_actions = IntCounterVec::new(
		Opts::new(
			"api_user_action_total",
			"User-triggered actions on matches and suggestions",
		),
		&["entity_type", "action"],
	)?;
	registry.register(Box::new(user_actions.clone()))?;
	USER_ACTIONS
		.set(user_actions)
		.map_err(|_| anyhow::anyhow!("user action metrics already initialised"))?;

	let user_agents = IntCounterVec::new(
		Opts::new(
			"api_user_agent_total",
			"Incoming requests by classified user agent and version",
		),
		&["product", "version"],
	)?;
	registry.register(Box::new(user_agents.clone()))?;
	USER_AGENTS
		.set(user_agents)
		.map_err(|_| anyhow::anyhow!("user agent metrics already initialised"))?;

	let launchbox_import_records = IntCounterVec::new(
		Opts::new(
			"api_launchbox_import_records_total",
			"LaunchBox bulk metadata import records by type and outcome",
		),
		&["record_type", "outcome"],
	)?;
	registry.register(Box::new(launchbox_import_records.clone()))?;
	LAUNCHBOX_IMPORT_RECORDS
		.set(launchbox_import_records)
		.map_err(|_| anyhow::anyhow!("launchbox import metrics already initialised"))?;

	let openvgdb_import_records = IntCounterVec::new(
		Opts::new(
			"api_openvgdb_import_records_total",
			"OpenVGDB bulk metadata import records by type and outcome",
		),
		&["record_type", "outcome"],
	)?;
	registry.register(Box::new(openvgdb_import_records.clone()))?;
	OPENVGDB_IMPORT_RECORDS
		.set(openvgdb_import_records)
		.map_err(|_| anyhow::anyhow!("openvgdb import metrics already initialised"))?;

	let retroachievements_import_records = IntCounterVec::new(
		Opts::new(
			"api_retroachievements_import_records_total",
			"RetroAchievements bulk metadata import records by type and outcome",
		),
		&["record_type", "outcome"],
	)?;
	registry.register(Box::new(retroachievements_import_records.clone()))?;
	RETROACHIEVEMENTS_IMPORT_RECORDS
		.set(retroachievements_import_records)
		.map_err(|_| anyhow::anyhow!("retroachievements import metrics already initialised"))?;

	let screenscraper_quota_exhaustion = IntCounterVec::new(
		Opts::new(
			"api_screenscraper_quota_exhaustion_total",
			"ScreenScraper quota exhaustion events by trigger",
		),
		&["trigger"],
	)?;
	registry.register(Box::new(screenscraper_quota_exhaustion.clone()))?;
	SCREENSCRAPER_QUOTA_EXHAUSTION
		.set(screenscraper_quota_exhaustion)
		.map_err(|_| anyhow::anyhow!("screenscraper quota metrics already initialised"))?;

	let metadata_request_inflight = IntGaugeVec::new(
		Opts::new(
			"api_metadata_request_inflight",
			"Current outbound metadata-provider HTTP requests in flight per provider",
		),
		&["provider"],
	)?;
	registry.register(Box::new(metadata_request_inflight.clone()))?;
	METADATA_REQUEST_INFLIGHT
		.set(metadata_request_inflight)
		.map_err(|_| anyhow::anyhow!("metadata request inflight metrics already initialised"))?;

	let metadata_request_attempts = IntCounterVec::new(
		Opts::new(
			"api_metadata_request_attempts_total",
			"HTTP attempts per metadata request labeled by per-attempt outcome (initial, retry, giveup)",
		),
		&["provider", "outcome"],
	)?;
	registry.register(Box::new(metadata_request_attempts.clone()))?;
	METADATA_REQUEST_ATTEMPTS
		.set(metadata_request_attempts)
		.map_err(|_| anyhow::anyhow!("metadata request attempts metrics already initialised"))?;

	let provider_concurrency_configured = IntGaugeVec::new(
		Opts::new(
			"api_provider_concurrency_configured",
			"Configured concurrency cap per metadata provider (static or last-known dynamic value)",
		),
		&["provider"],
	)?;
	registry.register(Box::new(provider_concurrency_configured.clone()))?;
	PROVIDER_CONCURRENCY_CONFIGURED
		.set(provider_concurrency_configured)
		.map_err(|_| anyhow::anyhow!("provider concurrency metrics already initialised"))?;

	let cross_match_attempts = IntCounterVec::new(
		Opts::new(
			"api_cross_match_attempts_total",
			"Cross-provider name match pass attempts by provider and outcome",
		),
		&["provider", "outcome"],
	)?;
	registry.register(Box::new(cross_match_attempts.clone()))?;
	CROSS_MATCH_ATTEMPTS
		.set(cross_match_attempts)
		.map_err(|_| anyhow::anyhow!("cross match metrics already initialised"))?;

	let thegamesdb_api_calls = IntCounterVec::new(
		Opts::new(
			"api_thegamesdb_api_calls_total",
			"TheGamesDB outbound API call outcomes (hit, miss, quota_exhausted, probe, error)",
		),
		&["outcome"],
	)?;
	registry.register(Box::new(thegamesdb_api_calls.clone()))?;
	THEGAMESDB_API_CALLS
		.set(thegamesdb_api_calls)
		.map_err(|_| anyhow::anyhow!("thegamesdb api metrics already initialised"))?;

	let thegamesdb_cycle_calls = IntGauge::new(
		"api_thegamesdb_cycle_calls_current",
		"TheGamesDB outbound calls used in the current daily match cycle (capped at PER_CYCLE_CAP)",
	)?;
	registry.register(Box::new(thegamesdb_cycle_calls.clone()))?;
	THEGAMESDB_CYCLE_CALLS
		.set(thegamesdb_cycle_calls)
		.map_err(|_| anyhow::anyhow!("thegamesdb cycle calls metrics already initialised"))?;

	let thegamesdb_remaining_allowance = IntGauge::new(
		"api_thegamesdb_remaining_allowance",
		"TheGamesDB remaining monthly API allowance reported by upstream (-1 when unknown)",
	)?;
	registry.register(Box::new(thegamesdb_remaining_allowance.clone()))?;
	THEGAMESDB_REMAINING_ALLOWANCE
		.set(thegamesdb_remaining_allowance)
		.map_err(|_| {
			anyhow::anyhow!("thegamesdb remaining allowance metrics already initialised")
		})?;

	let external_suggestion_queue_depth = IntGauge::new(
		"api_external_suggestion_queue_depth",
		"Current depth of the Redis-backed external suggestion queue (LLEN)",
	)?;
	registry.register(Box::new(external_suggestion_queue_depth.clone()))?;
	EXTERNAL_SUGGESTION_QUEUE_DEPTH
		.set(external_suggestion_queue_depth)
		.map_err(|_| anyhow::anyhow!("external suggestion queue metrics already initialised"))?;

	let http_rate_limit_rejected = IntCounterVec::new(
		Opts::new(
			"api_http_rate_limit_rejected_total",
			"Incoming HTTP requests rejected by the rate limiter, labelled by client classification",
		),
		&["client_classification"],
	)?;
	registry.register(Box::new(http_rate_limit_rejected.clone()))?;
	HTTP_RATE_LIMIT_REJECTED
		.set(http_rate_limit_rejected)
		.map_err(|_| anyhow::anyhow!("http rate limit rejected metrics already initialised"))?;

	let http_requests_inflight = IntGaugeVec::new(
		Opts::new(
			"api_http_requests_inflight",
			"Current in-flight inbound HTTP requests, labelled by matched route template and method",
		),
		&["route", "method"],
	)?;
	registry.register(Box::new(http_requests_inflight.clone()))?;
	HTTP_REQUESTS_INFLIGHT
		.set(http_requests_inflight)
		.map_err(|_| anyhow::anyhow!("http requests inflight metrics already initialised"))?;

	let db_pool_connections = IntGaugeVec::new(
		Opts::new(
			"api_db_pool_connections",
			"Postgres connection pool size by state (total, idle, active, max)",
		),
		&["state"],
	)?;
	registry.register(Box::new(db_pool_connections.clone()))?;
	DB_POOL_CONNECTIONS
		.set(db_pool_connections)
		.map_err(|_| anyhow::anyhow!("db pool metrics already initialised"))?;

	Ok(())
}

pub fn record_cache_hit(cache: &str, lookup: &str) {
	if let Some(counter) = CACHE_EVENTS.get() {
		counter.with_label_values(&[cache, lookup, "hit"]).inc();
	}
}

pub fn set_cache_l1_entries(lookup: &str, entries: u64) {
	if let Some(gauge) = CACHE_L1_SIZE.get() {
		gauge
			.with_label_values(&[lookup])
			.set(entries.min(i64::MAX as u64) as i64);
	}
}

pub fn record_cache_miss(cache: &str, lookup: &str) {
	if let Some(counter) = CACHE_EVENTS.get() {
		counter.with_label_values(&[cache, lookup, "miss"]).inc();
	}
}

pub fn record_identify_attempt(hash_type: &str, hit: bool) {
	if let Some(counter) = IDENTIFY_ATTEMPTS.get() {
		counter
			.with_label_values(&[hash_type, if hit { "hit" } else { "no_match" }])
			.inc();
	}
}

pub fn record_service_error(variant: &str) {
	if let Some(counter) = SERVICE_ERRORS.get() {
		counter.with_label_values(&[variant]).inc();
	}
}

pub fn record_metadata_auto_match(provider: &str, entity_type: &str, result: &str, reason: &str) {
	if let Some(counter) = METADATA_AUTO_MATCHES.get() {
		counter
			.with_label_values(&[provider, entity_type, result, reason])
			.inc();
	}
}

/// Per-rung outcome counter. `outcome` is one of `hit`, `ambiguous`, `miss`.
/// Together with `provider` and `rung` this lets dashboards compute
/// hit-rate per rung per provider.
pub fn record_match_rung(provider: &str, rung: &str, outcome: &str) {
	if let Some(counter) = MATCH_RUNG_OUTCOMES.get() {
		counter.with_label_values(&[provider, rung, outcome]).inc();
	}
}

pub fn record_metadata_token_refresh(provider: &str, trigger: &str, result: &str) {
	if let Some(counter) = METADATA_TOKEN_REFRESHES.get() {
		counter
			.with_label_values(&[provider, trigger, result])
			.inc();
	}
}

pub fn record_background_job(job: &str, result: &str, duration_seconds: f64) {
	if let Some(counter) = BACKGROUND_JOB_RUNS.get() {
		counter.with_label_values(&[job, result]).inc();
	}
	if let Some(histogram) = BACKGROUND_JOB_DURATION.get() {
		histogram
			.with_label_values(&[job])
			.observe(duration_seconds);
	}
	if result == "success"
		&& let Some(gauge) = BACKGROUND_JOB_LAST_SUCCESS.get()
		&& let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
	{
		gauge.with_label_values(&[job]).set(now.as_secs() as i64);
	}
}

pub fn record_metadata_request(
	provider: &str,
	endpoint: &str,
	status_class: &str,
	status_code: &str,
	duration_seconds: f64,
) {
	if let Some(counter) = METADATA_REQUESTS.get() {
		counter
			.with_label_values(&[provider, endpoint, status_class, status_code])
			.inc();
	}
	if let Some(histogram) = METADATA_REQUEST_DURATION.get() {
		histogram
			.with_label_values(&[provider, endpoint, status_class, status_code])
			.observe(duration_seconds);
	}
}

pub fn record_dat_ingestion_file(source: &str, outcome: &str) {
	if let Some(counter) = DAT_INGESTION_FILES.get() {
		counter.with_label_values(&[source, outcome]).inc();
	}
}

pub fn record_clone_of_resolution(result: &str) {
	if let Some(counter) = CLONE_OF_RESOLUTIONS.get() {
		counter.with_label_values(&[result]).inc();
	}
}

pub fn record_user_action(entity_type: &str, action: &str) {
	if let Some(counter) = USER_ACTIONS.get() {
		counter.with_label_values(&[entity_type, action]).inc();
	}
}

pub fn record_user_agent(product: &str, version: &str) {
	if let Some(counter) = USER_AGENTS.get() {
		counter.with_label_values(&[product, version]).inc();
	}
}

pub fn record_launchbox_import_records(record_type: &str, outcome: &str, n: u64) {
	if let Some(counter) = LAUNCHBOX_IMPORT_RECORDS.get() {
		counter.with_label_values(&[record_type, outcome]).inc_by(n);
	}
}

pub fn record_openvgdb_import_records(record_type: &str, outcome: &str, n: u64) {
	if let Some(counter) = OPENVGDB_IMPORT_RECORDS.get() {
		counter.with_label_values(&[record_type, outcome]).inc_by(n);
	}
}

pub fn record_retroachievements_import_records(record_type: &str, outcome: &str, n: u64) {
	if let Some(counter) = RETROACHIEVEMENTS_IMPORT_RECORDS.get() {
		counter.with_label_values(&[record_type, outcome]).inc_by(n);
	}
}

pub fn record_screenscraper_quota_exhaustion(trigger: &str) {
	if let Some(counter) = SCREENSCRAPER_QUOTA_EXHAUSTION.get() {
		counter.with_label_values(&[trigger]).inc();
	}
}

pub fn metadata_request_inflight_inc(provider: &str) {
	if let Some(gauge) = METADATA_REQUEST_INFLIGHT.get() {
		gauge.with_label_values(&[provider]).inc();
	}
}

pub fn metadata_request_inflight_dec(provider: &str) {
	if let Some(gauge) = METADATA_REQUEST_INFLIGHT.get() {
		gauge.with_label_values(&[provider]).dec();
	}
}

pub fn record_metadata_request_attempt(provider: &str, outcome: &str) {
	if let Some(counter) = METADATA_REQUEST_ATTEMPTS.get() {
		counter.with_label_values(&[provider, outcome]).inc();
	}
}

pub fn set_provider_concurrency_configured(provider: &str, value: i64) {
	if let Some(gauge) = PROVIDER_CONCURRENCY_CONFIGURED.get() {
		gauge.with_label_values(&[provider]).set(value);
	}
}

pub fn record_cross_match_attempt(provider: &str, outcome: &str) {
	if let Some(counter) = CROSS_MATCH_ATTEMPTS.get() {
		counter.with_label_values(&[provider, outcome]).inc();
	}
}

pub fn set_thegamesdb_cycle_calls(value: u32) {
	if let Some(gauge) = THEGAMESDB_CYCLE_CALLS.get() {
		gauge.set(value as i64);
	}
}

pub fn set_thegamesdb_remaining_allowance(value: i32) {
	if let Some(gauge) = THEGAMESDB_REMAINING_ALLOWANCE.get() {
		gauge.set(value as i64);
	}
}

pub fn set_external_suggestion_queue_depth(depth: i64) {
	if let Some(gauge) = EXTERNAL_SUGGESTION_QUEUE_DEPTH.get() {
		gauge.set(depth);
	}
}

pub fn record_http_rate_limit_rejected(client_classification: &str) {
	if let Some(counter) = HTTP_RATE_LIMIT_REJECTED.get() {
		counter.with_label_values(&[client_classification]).inc();
	}
}

pub fn http_requests_inflight_inc(route: &str, method: &str) {
	if let Some(gauge) = HTTP_REQUESTS_INFLIGHT.get() {
		gauge.with_label_values(&[route, method]).inc();
	}
}

pub fn http_requests_inflight_dec(route: &str, method: &str) {
	if let Some(gauge) = HTTP_REQUESTS_INFLIGHT.get() {
		gauge.with_label_values(&[route, method]).dec();
	}
}

pub fn set_db_pool_connections(state: &str, value: i64) {
	if let Some(gauge) = DB_POOL_CONNECTIONS.get() {
		gauge.with_label_values(&[state]).set(value);
	}
}

pub fn record_thegamesdb_api_call(outcome: &str) {
	if let Some(counter) = THEGAMESDB_API_CALLS.get() {
		counter.with_label_values(&[outcome]).inc();
	}
}
