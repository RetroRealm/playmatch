use prometheus::{IntCounterVec, Opts, Registry};
use std::sync::OnceLock;

static CACHE_EVENTS: OnceLock<IntCounterVec> = OnceLock::new();
static IDENTIFY_ATTEMPTS: OnceLock<IntCounterVec> = OnceLock::new();
static SERVICE_ERRORS: OnceLock<IntCounterVec> = OnceLock::new();
static IGDB_AUTO_MATCHES: OnceLock<IntCounterVec> = OnceLock::new();
static IGDB_TOKEN_REFRESHES: OnceLock<IntCounterVec> = OnceLock::new();

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
