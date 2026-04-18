use prometheus::{HistogramOpts, HistogramVec, IntCounterVec, Opts, Registry};
use std::sync::OnceLock;

static CACHE_EVENTS: OnceLock<IntCounterVec> = OnceLock::new();
static IDENTIFY_ATTEMPTS: OnceLock<IntCounterVec> = OnceLock::new();
static SERVICE_ERRORS: OnceLock<IntCounterVec> = OnceLock::new();
static IGDB_AUTO_MATCHES: OnceLock<IntCounterVec> = OnceLock::new();
static IGDB_TOKEN_REFRESHES: OnceLock<IntCounterVec> = OnceLock::new();
static BACKGROUND_JOB_RUNS: OnceLock<IntCounterVec> = OnceLock::new();
static BACKGROUND_JOB_DURATION: OnceLock<HistogramVec> = OnceLock::new();
static IGDB_REQUESTS: OnceLock<IntCounterVec> = OnceLock::new();
static IGDB_REQUEST_DURATION: OnceLock<HistogramVec> = OnceLock::new();
static DAT_INGESTION_FILES: OnceLock<IntCounterVec> = OnceLock::new();
static CLONE_OF_RESOLUTIONS: OnceLock<IntCounterVec> = OnceLock::new();
static USER_ACTIONS: OnceLock<IntCounterVec> = OnceLock::new();
static USER_AGENTS: OnceLock<IntCounterVec> = OnceLock::new();

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

	let igdb_auto_matches = IntCounterVec::new(
		Opts::new(
			"api_igdb_auto_match_total",
			"Automatic IGDB match outcomes per entity type, result and reason",
		),
		&["entity_type", "result", "reason"],
	)?;
	registry.register(Box::new(igdb_auto_matches.clone()))?;
	IGDB_AUTO_MATCHES
		.set(igdb_auto_matches)
		.map_err(|_| anyhow::anyhow!("igdb auto match metrics already initialised"))?;

	let igdb_token_refreshes = IntCounterVec::new(
		Opts::new(
			"api_igdb_token_refresh_total",
			"IGDB OAuth2 token refresh attempts by trigger and result",
		),
		&["trigger", "result"],
	)?;
	registry.register(Box::new(igdb_token_refreshes.clone()))?;
	IGDB_TOKEN_REFRESHES
		.set(igdb_token_refreshes)
		.map_err(|_| anyhow::anyhow!("igdb token refresh metrics already initialised"))?;

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

	let igdb_requests = IntCounterVec::new(
		Opts::new(
			"api_igdb_request_total",
			"IGDB outbound requests by endpoint and result",
		),
		&["endpoint", "result"],
	)?;
	registry.register(Box::new(igdb_requests.clone()))?;
	IGDB_REQUESTS
		.set(igdb_requests)
		.map_err(|_| anyhow::anyhow!("igdb request metrics already initialised"))?;

	let igdb_request_duration = HistogramVec::new(
		HistogramOpts::new(
			"api_igdb_request_duration_seconds",
			"IGDB outbound request duration in seconds",
		)
		.buckets(vec![
			0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0,
		]),
		&["endpoint"],
	)?;
	registry.register(Box::new(igdb_request_duration.clone()))?;
	IGDB_REQUEST_DURATION
		.set(igdb_request_duration)
		.map_err(|_| anyhow::anyhow!("igdb request duration metrics already initialised"))?;

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

	Ok(())
}

pub fn record_cache_hit(cache: &str, lookup: &str) {
	if let Some(counter) = CACHE_EVENTS.get() {
		counter.with_label_values(&[cache, lookup, "hit"]).inc();
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

pub fn record_igdb_auto_match(entity_type: &str, result: &str, reason: &str) {
	if let Some(counter) = IGDB_AUTO_MATCHES.get() {
		counter
			.with_label_values(&[entity_type, result, reason])
			.inc();
	}
}

pub fn record_igdb_token_refresh(trigger: &str, result: &str) {
	if let Some(counter) = IGDB_TOKEN_REFRESHES.get() {
		counter.with_label_values(&[trigger, result]).inc();
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
}

pub fn record_igdb_request(endpoint: &str, result: &str, duration_seconds: f64) {
	if let Some(counter) = IGDB_REQUESTS.get() {
		counter.with_label_values(&[endpoint, result]).inc();
	}
	if let Some(histogram) = IGDB_REQUEST_DURATION.get() {
		histogram
			.with_label_values(&[endpoint])
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
