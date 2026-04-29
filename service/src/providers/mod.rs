pub mod emuready;
pub mod igdb;
pub mod launchbox;
pub mod mobygames;
pub mod screenscraper;
pub mod steamgriddb;

use crate::db::signature_metadata_mapping::{
	SignatureMetadataMappingInputBuilder, create_or_update_signature_metadata_mapping,
};
use crate::metrics::{record_background_job, record_metadata_auto_match};
use entity::sea_orm_active_enums::{
	AutomaticMatchReasonEnum, FailedMatchReasonEnum, MatchTypeEnum, MetadataProviderEnum,
};
use futures_util::future::BoxFuture;
use log::{error, info};
use sea_orm::DbConn;
use sea_orm::prelude::Uuid;
use std::sync::Arc;
use std::time::Instant;

/// Tuned for IGDB's 4 req/s rate limit. Providers with a different rate
/// budget should override [`MetadataProvider::chunk_size`].
pub const DEFAULT_CHUNK_SIZE: usize = 4;

pub const DEFAULT_PAGE_SIZE: u64 = 100;

/// Function pointer signature for the per-page fetch callback used by
/// [`drive_match_pipeline`]. Returns the next page of unmatched entities of
/// type `M` for the given provider, or `None` when the queue is drained.
pub type FetchPageFn<M> =
	fn(MetadataProviderEnum, u64, DbConn) -> BoxFuture<'static, anyhow::Result<Option<Vec<M>>>>;

/// Function pointer signature for the per-entity match callback used by
/// [`drive_match_pipeline`]. Receives one entity, an `Arc` clone of the
/// provider client, and an owned `DbConn`.
pub type MatchEntityFn<M, C> = fn(M, Arc<C>, DbConn) -> BoxFuture<'static, anyhow::Result<()>>;

/// Function pointer signature for the per-entity callback used by
/// [`drive_cross_match_pipeline`]. Receives one entity, the deduped sibling
/// matched-names to attempt, an `Arc` clone of the provider client, and an
/// owned `DbConn`.
pub type CrossMatchFn<C> =
	fn(entity::game::Model, Vec<String>, Arc<C>, DbConn) -> BoxFuture<'static, anyhow::Result<()>>;

/// Identifies which entity table a match write targets. Carries the FK value
/// so the writer helpers can populate the matching column on the
/// `signature_metadata_mapping` row.
#[derive(Debug, Clone, Copy)]
pub enum Target {
	Company(Uuid),
	Platform(Uuid),
	Game(Uuid),
}

impl Target {
	pub fn entity_label(self) -> &'static str {
		match self {
			Self::Company(_) => "company",
			Self::Platform(_) => "platform",
			Self::Game(_) => "game",
		}
	}

	fn apply(
		self,
		b: &mut SignatureMetadataMappingInputBuilder,
	) -> &mut SignatureMetadataMappingInputBuilder {
		match self {
			Self::Company(id) => b.company_id(Some(id)),
			Self::Platform(id) => b.platform_id(Some(id)),
			Self::Game(id) => b.game_id(Some(id)),
		}
	}
}

#[allow(clippy::too_many_arguments)]
pub async fn write_auto_match_success(
	provider_label: &'static str,
	provider_enum: MetadataProviderEnum,
	target: Target,
	provider_id: String,
	reason: AutomaticMatchReasonEnum,
	matched_name: Option<String>,
	db_conn: &DbConn,
	redis_conn: &mut redis::aio::MultiplexedConnection,
) -> anyhow::Result<()> {
	let mut builder = SignatureMetadataMappingInputBuilder::default();
	target.apply(&mut builder);
	let input = builder
		.provider(provider_enum)
		.provider_id(Some(provider_id))
		.match_type(MatchTypeEnum::Automatic)
		.automatic_match_reason(Some(reason))
		.matched_name(matched_name)
		.build()?;
	create_or_update_signature_metadata_mapping(input, db_conn).await?;
	if let Target::Game(game_id) = target {
		crate::identification::cache::bust_identify_cache_for_game(redis_conn, db_conn, game_id)
			.await?;
	}
	record_metadata_auto_match(
		provider_label,
		target.entity_label(),
		"matched",
		automatic_reason_label(reason),
	);
	Ok(())
}

pub async fn write_auto_match_failed(
	provider_label: &'static str,
	provider_enum: MetadataProviderEnum,
	target: Target,
	reason: FailedMatchReasonEnum,
	db_conn: &DbConn,
	redis_conn: &mut redis::aio::MultiplexedConnection,
) -> anyhow::Result<()> {
	let mut builder = SignatureMetadataMappingInputBuilder::default();
	target.apply(&mut builder);
	let input = builder
		.provider(provider_enum)
		.match_type(MatchTypeEnum::Failed)
		.failed_match_reason(Some(reason))
		.build()?;
	create_or_update_signature_metadata_mapping(input, db_conn).await?;
	if let Target::Game(game_id) = target {
		crate::identification::cache::bust_identify_cache_for_game(redis_conn, db_conn, game_id)
			.await?;
	}
	record_metadata_auto_match(
		provider_label,
		target.entity_label(),
		"failed",
		failed_reason_label(reason),
	);
	Ok(())
}

/// Hand-written so that adding a new variant to [`AutomaticMatchReasonEnum`]
/// produces a compile error here. Keep in sync with the dashboard contract.
fn automatic_reason_label(r: AutomaticMatchReasonEnum) -> &'static str {
	match r {
		AutomaticMatchReasonEnum::DirectName => "direct_name",
		AutomaticMatchReasonEnum::AlternativeName => "alternative_name",
		AutomaticMatchReasonEnum::ViaChild => "via_child",
		AutomaticMatchReasonEnum::ViaParent => "via_parent",
		AutomaticMatchReasonEnum::NormalizedName => "normalized_name",
		AutomaticMatchReasonEnum::NormalizedAlternativeName => "normalized_alternative_name",
		AutomaticMatchReasonEnum::Md5Hash => "md5_hash",
		AutomaticMatchReasonEnum::Sha1Hash => "sha1_hash",
		AutomaticMatchReasonEnum::CrcHash => "crc_hash",
		AutomaticMatchReasonEnum::CrossProviderDirectName => "cross_provider_direct_name",
		AutomaticMatchReasonEnum::CrossProviderNormalizedName => "cross_provider_normalized_name",
	}
}

/// Hand-written so that adding a new variant to [`FailedMatchReasonEnum`]
/// produces a compile error here. Keep in sync with the dashboard contract.
fn failed_reason_label(r: FailedMatchReasonEnum) -> &'static str {
	match r {
		FailedMatchReasonEnum::NoDirectMatch => "no_direct_match",
		FailedMatchReasonEnum::TooManyMatches => "too_many_matches",
	}
}

/// Drive a "page > chunk > spawn > join" matching loop. `fetch_fn` returns
/// the next page of unmatched entities, or `None` when drained. `match_fn`
/// runs per entity on its own task with a clone of `client` and `db_conn`.
/// Per-entity errors are logged with the `label` prefix and do not abort
/// the loop.
pub async fn drive_match_pipeline<M, C>(
	label: &'static str,
	provider: MetadataProviderEnum,
	fetch_fn: FetchPageFn<M>,
	match_fn: MatchEntityFn<M, C>,
	client: Arc<C>,
	db_conn: &DbConn,
	chunk_size: usize,
) -> anyhow::Result<()>
where
	M: Clone + Send + 'static,
	C: Send + Sync + 'static,
{
	while let Some(page) = fetch_fn(provider, DEFAULT_PAGE_SIZE, db_conn.clone()).await? {
		for chunk in page.chunks(chunk_size) {
			let mut handles = Vec::with_capacity(chunk.len());
			for entity in chunk.iter().cloned() {
				let client = client.clone();
				let conn = db_conn.clone();
				handles.push(tokio::spawn(match_fn(entity, client, conn)));
			}
			for handle in handles {
				if let Err(e) = handle.await? {
					error!("Error while matching {label} to provider: {e:?}");
				}
			}
		}
	}
	Ok(())
}

/// Drive a cross-provider name retry pass for `provider`. Pages through
/// games that this provider failed to match while at least one sibling
/// provider has a non-null `matched_name`, then dispatches `match_fn` per
/// game with the deduped sibling names.
///
/// After each chunk completes the page's failed mappings get
/// `cross_match_last_tried_at` stamped via a single bulk UPDATE. That is
/// the load-bearing piece: without it, the next page-fetch would return
/// the same games again because non-matching cross-pass attempts leave
/// the row in `Failed` state. With the stamp + the
/// [`crate::db::game::CROSS_MATCH_RETRY_INTERVAL_DAYS`] cooldown filter,
/// each game gets at most one cross-pass attempt per cycle and one
/// retry per week.
///
/// Per-entity errors are logged and do not abort the loop.
pub async fn drive_cross_match_pipeline<C>(
	label: &'static str,
	provider: MetadataProviderEnum,
	match_fn: CrossMatchFn<C>,
	client: Arc<C>,
	db_conn: &DbConn,
	chunk_size: usize,
) -> anyhow::Result<()>
where
	C: Send + Sync + 'static,
{
	while let Some(page) = crate::db::game::get_failed_games_for_cross_pass_with_limit(
		provider,
		DEFAULT_PAGE_SIZE,
		db_conn.clone(),
	)
	.await?
	{
		for chunk in page.chunks(chunk_size) {
			let mut handles = Vec::with_capacity(chunk.len());
			for game in chunk.iter().cloned() {
				let client = client.clone();
				let conn = db_conn.clone();
				handles.push(tokio::spawn(async move {
					let siblings =
						crate::db::signature_metadata_mapping::find_sibling_matched_names(
							game.id, provider, &conn,
						)
						.await?;
					if siblings.is_empty() {
						return Ok(());
					}
					let names: Vec<String> = siblings.into_iter().map(|(_, n)| n).collect();
					match_fn(game, names, client, conn).await
				}));
			}
			for handle in handles {
				if let Err(e) = handle.await? {
					error!("Error while cross-matching {label} to provider: {e:?}");
				}
			}
			let chunk_ids: Vec<Uuid> = chunk.iter().map(|g| g.id).collect();
			if let Err(e) = crate::db::signature_metadata_mapping::bulk_stamp_cross_match_attempt(
				provider, &chunk_ids, db_conn,
			)
			.await
			{
				error!(
					"Failed to stamp cross_match_last_tried_at for {} {label} mappings: {e}",
					chunk_ids.len()
				);
			}
		}
	}
	Ok(())
}

/// Implemented by every metadata provider that participates in the
/// background match cron.
///
/// Uses `async_trait` rather than native AFIT because the trait is
/// dyn-dispatched via `Arc<dyn MetadataProvider>` and native AFIT is not
/// dyn-safe.
#[async_trait::async_trait]
pub trait MetadataProvider: Send + Sync + 'static {
	/// Stable identifier used for the metric label, the cache namespace, and
	/// the env-var prefix. Lowercase. Must match the `string_value` in
	/// [`MetadataProviderEnum`].
	fn provider_label(&self) -> &'static str;

	/// The corresponding sea-orm enum variant.
	fn provider_enum(&self) -> MetadataProviderEnum;

	/// Override when the provider's rate budget differs from IGDB's.
	fn chunk_size(&self) -> usize {
		DEFAULT_CHUNK_SIZE
	}

	/// Run a full match cycle for this provider. Owns its own page/chunk/spawn
	/// pipeline so providers can express custom strategies (parent/child
	/// propagation, retry passes) instead of fitting a uniform driver.
	///
	/// `Arc<Self>` receiver so the impl can hand the same `Arc` into
	/// `tokio::spawn` without requiring `Self: Clone`.
	async fn match_db(self: Arc<Self>, db_conn: &DbConn) -> anyhow::Result<()>;

	/// Cross-provider name retry pass. Default no-op so providers can opt in
	/// individually. Runs as a second wave after every provider's primary
	/// `match_db` has finished, so each provider's cross-pass sees the full
	/// set of sibling `matched_name` values regardless of registry order.
	async fn match_via_sibling_names(self: Arc<Self>, _db_conn: &DbConn) -> anyhow::Result<()> {
		Ok(())
	}
}

/// Cron runs providers in registration order.
pub type ProviderRegistry = Vec<Arc<dyn MetadataProvider>>;

/// Per-provider failures are logged and recorded as `{label}_match` failure
/// metrics; they do not stop other providers. After every provider's primary
/// cycle finishes, a second wave runs `match_via_sibling_names` so each
/// provider can use the full set of sibling matched_names recorded across
/// the registry.
pub async fn match_db_to_all_providers(
	registry: &ProviderRegistry,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	let aggregate_started = Instant::now();
	for provider in registry {
		let label = provider.provider_label();
		info!("Starting match cycle for provider '{label}'");
		let started = Instant::now();
		let result = match provider.clone().match_db(db_conn).await {
			Ok(()) => {
				info!("Finished match cycle for provider '{label}'");
				"success"
			}
			Err(e) => {
				error!("Provider '{label}' match cycle failed: {e:?}");
				"failure"
			}
		};
		record_background_job(
			&format!("{label}_match"),
			result,
			started.elapsed().as_secs_f64(),
		);
	}

	for provider in registry {
		let label = provider.provider_label();
		info!("Starting cross-provider name retry pass for '{label}'");
		let started = Instant::now();
		let result = match provider.clone().match_via_sibling_names(db_conn).await {
			Ok(()) => {
				info!("Finished cross-provider name retry pass for '{label}'");
				"success"
			}
			Err(e) => {
				error!("Provider '{label}' cross-provider name pass failed: {e:?}");
				"failure"
			}
		};
		record_background_job(
			&format!("{label}_cross_match"),
			result,
			started.elapsed().as_secs_f64(),
		);
	}

	record_background_job(
		"provider_match_all",
		"success",
		aggregate_started.elapsed().as_secs_f64(),
	);
	Ok(())
}
