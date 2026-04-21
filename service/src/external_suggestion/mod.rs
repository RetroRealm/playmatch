use crate::db::game::{
	find_game_and_id_mapping_by_md5, find_game_and_id_mapping_by_sha1,
	find_game_and_id_mapping_by_sha256, find_game_by_name_or_game_file_name,
};
use crate::db::signature_metadata_mapping::find_signature_metadata_mapping_by_platform_game_company_and_provider;
use crate::db::signature_metadata_mapping_suggestions::{insert_suggestion, suggestion_exists};
use crate::error::ServiceResult;
use crate::model::MAX_NAME_INPUT_LEN;
use crate::model::external_suggestion::{
	ExternalGameMatchSuggestionPayload, ExternalProviderMapping, QueuedExternalSuggestion,
};
use crate::model::validate_optional_hex;
use chrono::Utc;
use entity::sea_orm_active_enums::{MatchTypeEnum, MetadataProviderEnum};
use entity::signature_metadata_mapping_suggestions::ActiveModel;
use log::{debug, warn};
use redis::AsyncTypedCommands;
use redis::aio::MultiplexedConnection;
use sea_orm::{DbConn, Set};
use serde::{Deserialize, Serialize};

const QUEUE_KEY: &str = "playmatch:queue:external_suggestion:v1";
const QUEUE_SOFT_CAP: usize = 10_000;
const RATE_LIMIT_PER_MINUTE: isize = 20;
const RATE_LIMIT_WINDOW_SECS: i64 = 60;
const RATE_LIMIT_KEY_PREFIX: &str = "playmatch:ratelimit:external_suggestion:";
const USER_AGENT_MAX_LEN: usize = 255;
const PROVIDER_ID_MAX_LEN: usize = MAX_NAME_INPUT_LEN;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnqueueOutcome {
	Accepted,
	RateLimited,
	QueueFull,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessOutcome {
	InvalidPayload,
	UnknownRom,
	InvalidMapping,
	UnsupportedProvider,
	AlreadyMatched,
	DuplicateSuggestion,
	Created,
}

impl ProcessOutcome {
	fn metric_label(self) -> &'static str {
		match self {
			Self::InvalidPayload => "invalid_payload",
			Self::UnknownRom => "unknown_rom",
			Self::InvalidMapping => "invalid_mapping",
			Self::UnsupportedProvider => "unsupported_provider",
			Self::AlreadyMatched => "already_matched",
			Self::DuplicateSuggestion => "duplicate_suggestion",
			Self::Created => "created",
		}
	}
}

fn record(outcome: ProcessOutcome) {
	crate::metrics::record_user_action("external_suggestion", outcome.metric_label());
}

/// Empty or whitespace-only input becomes `None` so callers can persist `NULL`.
pub fn truncate_user_agent(ua: Option<&str>) -> Option<String> {
	let trimmed = ua?.trim();
	if trimmed.is_empty() {
		return None;
	}
	let mut out = String::with_capacity(trimmed.len().min(USER_AGENT_MAX_LEN));
	for c in trimmed.chars() {
		if out.len() + c.len_utf8() > USER_AGENT_MAX_LEN {
			break;
		}
		out.push(c);
	}
	Some(out)
}

/// Returns `true` while the caller is under the cap for the current 60s window.
pub async fn check_rate_limit(
	redis_conn: &mut MultiplexedConnection,
	ip: &str,
) -> ServiceResult<bool> {
	let key = format!("{RATE_LIMIT_KEY_PREFIX}{ip}");
	let count = redis_conn.incr(&key, 1).await?;
	if count == 1
		&& let Err(e) = redis_conn.expire(&key, RATE_LIMIT_WINDOW_SECS).await
	{
		warn!("rate-limit expire failed for {key}: {e}");
	}
	Ok(count <= RATE_LIMIT_PER_MINUTE)
}

pub async fn enqueue_external_suggestion(
	redis_conn: &mut MultiplexedConnection,
	payload: ExternalGameMatchSuggestionPayload,
	user_agent: Option<String>,
) -> ServiceResult<EnqueueOutcome> {
	let len = redis_conn.llen(QUEUE_KEY).await.unwrap_or(0);
	if len >= QUEUE_SOFT_CAP {
		warn!("external suggestion queue soft cap reached at {len}, dropping payload");
		return Ok(EnqueueOutcome::QueueFull);
	}

	let envelope = QueuedExternalSuggestion {
		payload,
		user_agent,
		enqueued_at: Utc::now(),
	};
	let encoded = serde_json::to_string(&envelope)?;
	redis_conn.rpush(QUEUE_KEY, encoded).await?;
	Ok(EnqueueOutcome::Accepted)
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DrainStats {
	pub processed_envelopes: u32,
	pub invalid_payloads: u32,
	pub unknown_roms: u32,
	pub invalid_mappings: u32,
	pub unsupported_providers: u32,
	pub already_matched: u32,
	pub duplicate_suggestions: u32,
	pub created: u32,
}

impl DrainStats {
	fn record(&mut self, outcome: ProcessOutcome) {
		match outcome {
			ProcessOutcome::InvalidPayload => self.invalid_payloads += 1,
			ProcessOutcome::UnknownRom => self.unknown_roms += 1,
			ProcessOutcome::InvalidMapping => self.invalid_mappings += 1,
			ProcessOutcome::UnsupportedProvider => self.unsupported_providers += 1,
			ProcessOutcome::AlreadyMatched => self.already_matched += 1,
			ProcessOutcome::DuplicateSuggestion => self.duplicate_suggestions += 1,
			ProcessOutcome::Created => self.created += 1,
		}
		record(outcome);
	}
}

pub async fn drain_external_suggestions(
	batch_limit: usize,
	db_conn: &DbConn,
	redis_conn: &mut MultiplexedConnection,
) -> ServiceResult<DrainStats> {
	let mut stats = DrainStats::default();

	for _ in 0..batch_limit {
		let popped: Option<String> = match redis_conn.lpop(QUEUE_KEY, None).await {
			Ok(v) => v,
			Err(e) => {
				warn!("external suggestion queue pop failed: {e}");
				break;
			}
		};
		let Some(raw) = popped else { break };

		stats.processed_envelopes += 1;
		let envelope = match serde_json::from_str::<QueuedExternalSuggestion>(&raw) {
			Ok(e) => e,
			Err(e) => {
				let preview: String = raw.chars().take(200).collect();
				warn!("dropping unparseable external suggestion envelope ({e}): {preview}");
				stats.record(ProcessOutcome::InvalidPayload);
				continue;
			}
		};

		process_envelope(envelope, db_conn, &mut stats).await;
	}

	Ok(stats)
}

async fn process_envelope(
	envelope: QueuedExternalSuggestion,
	db_conn: &DbConn,
	stats: &mut DrainStats,
) {
	if let Err(reason) = validate_envelope_basics(&envelope.payload) {
		debug!("external suggestion rejected: {reason}");
		stats.record(ProcessOutcome::InvalidPayload);
		return;
	}

	let game = match resolve_game(&envelope.payload, db_conn).await {
		Ok(Some(g)) => g,
		Ok(None) => {
			stats.record(ProcessOutcome::UnknownRom);
			return;
		}
		Err(e) => {
			warn!("db error while resolving external suggestion game: {e}");
			stats.record(ProcessOutcome::UnknownRom);
			return;
		}
	};

	for mapping in envelope.payload.mappings {
		process_mapping(
			game.id,
			mapping,
			envelope.user_agent.clone(),
			db_conn,
			stats,
		)
		.await;
	}
}

fn validate_envelope_basics(payload: &ExternalGameMatchSuggestionPayload) -> Result<(), String> {
	validate_optional_hex(&payload.md5, 32, "md5")?;
	validate_optional_hex(&payload.sha1, 40, "sha1")?;
	validate_optional_hex(&payload.sha256, 64, "sha256")?;

	let has_hash = payload.md5.is_some() || payload.sha1.is_some() || payload.sha256.is_some();
	let has_name = payload
		.file_name
		.as_deref()
		.map(str::trim)
		.map(|n| !n.is_empty())
		.unwrap_or(false);
	if !has_hash && !has_name {
		return Err("no hash or file name provided".to_string());
	}

	if payload.mappings.is_empty() {
		return Err("mappings must not be empty".to_string());
	}
	Ok(())
}

async fn resolve_game(
	payload: &ExternalGameMatchSuggestionPayload,
	db_conn: &DbConn,
) -> ServiceResult<Option<entity::game::Model>> {
	if let Some(sha256) = &payload.sha256
		&& let Some((game, _)) = find_game_and_id_mapping_by_sha256(sha256, db_conn).await?
	{
		return Ok(Some(game));
	}
	if let Some(sha1) = &payload.sha1
		&& let Some((game, _)) = find_game_and_id_mapping_by_sha1(sha1, db_conn).await?
	{
		return Ok(Some(game));
	}
	if let Some(md5) = &payload.md5
		&& let Some((game, _)) = find_game_and_id_mapping_by_md5(md5, db_conn).await?
	{
		return Ok(Some(game));
	}
	if let Some(name) = payload.file_name.as_deref().map(str::trim)
		&& !name.is_empty()
		&& let Some(game) = find_game_by_name_or_game_file_name(name, db_conn).await?
	{
		return Ok(Some(game));
	}
	Ok(None)
}

fn parse_provider(raw: &str) -> Option<MetadataProviderEnum> {
	match raw.trim().to_ascii_uppercase().as_str() {
		"IGDB" => Some(MetadataProviderEnum::Igdb),
		_ => None,
	}
}

async fn process_mapping(
	game_id: sea_orm::prelude::Uuid,
	mapping: ExternalProviderMapping,
	source: Option<String>,
	db_conn: &DbConn,
	stats: &mut DrainStats,
) {
	let Some(provider) = parse_provider(&mapping.provider) else {
		debug!(
			"external suggestion: dropping mapping with unsupported provider '{}'",
			mapping.provider
		);
		stats.record(ProcessOutcome::UnsupportedProvider);
		return;
	};
	let provider_id = mapping.provider_id.trim().to_string();
	if provider_id.is_empty() || provider_id.len() > PROVIDER_ID_MAX_LEN {
		stats.record(ProcessOutcome::InvalidMapping);
		return;
	}

	let existing_mapping =
		match find_signature_metadata_mapping_by_platform_game_company_and_provider(
			None,
			Some(game_id),
			None,
			provider,
			db_conn,
		)
		.await
		{
			Ok(v) => v,
			Err(e) => {
				warn!("db error while checking existing mapping: {e}");
				stats.record(ProcessOutcome::AlreadyMatched);
				return;
			}
		};

	if let Some(existing) = existing_mapping
		&& !matches!(
			existing.match_type,
			MatchTypeEnum::Failed | MatchTypeEnum::None
		) {
		stats.record(ProcessOutcome::AlreadyMatched);
		return;
	}

	let exists = match suggestion_exists(
		Some(game_id),
		None,
		None,
		provider,
		provider_id.clone(),
		db_conn,
	)
	.await
	{
		Ok(v) => v,
		Err(e) => {
			warn!("db error while checking existing suggestion: {e}");
			stats.record(ProcessOutcome::DuplicateSuggestion);
			return;
		}
	};

	if exists {
		stats.record(ProcessOutcome::DuplicateSuggestion);
		return;
	}

	let active = ActiveModel {
		game_id: Set(Some(game_id)),
		provider: Set(provider),
		provider_id: Set(provider_id),
		comment: Set(None),
		created_by: Set(None),
		source: Set(source),
		..Default::default()
	};

	match insert_suggestion(active, db_conn).await {
		Ok(_) => stats.record(ProcessOutcome::Created),
		Err(e) => {
			warn!("failed to insert external suggestion: {e}");
			stats.record(ProcessOutcome::InvalidPayload);
		}
	}
}
