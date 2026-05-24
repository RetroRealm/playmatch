pub mod emuready;
pub mod igdb;
pub mod launchbox;
pub mod mobygames;
pub mod openvgdb;
pub mod retroachievements;
pub mod screenscraper;
pub mod steamgriddb;
pub mod thegamesdb;

use crate::db::game::{
	find_game_parent, has_outstanding_cross_match_work_for_provider,
	has_outstanding_match_work_for_provider,
};
use crate::db::signature_metadata_mapping::{
	SignatureMetadataMappingInputBuilder, create_or_update_signature_metadata_mapping,
	find_signature_metadata_mapping_by_platform_game_company_and_provider,
};
use crate::metrics::{record_background_job, record_metadata_auto_match};
use entity::sea_orm_active_enums::{
	AutomaticMatchReasonEnum, FailedMatchReasonEnum, MatchTypeEnum, MetadataProviderEnum,
};
use futures_util::future::BoxFuture;
use log::{debug, error, info};
use sea_orm::DbConn;
use sea_orm::prelude::Uuid;
use std::sync::Arc;
use std::time::Instant;
use tokio::task::JoinSet;

/// Tuned for IGDB's 4 req/s rate limit. Providers with a different rate
/// budget should override [`MetadataProvider::chunk_size`].
pub const DEFAULT_CHUNK_SIZE: usize = 4;

pub const DEFAULT_PAGE_SIZE: u64 = 100;

/// Exposes the primary-key UUID used as the keyset pagination cursor.
/// [`drive_match_pipeline`] reads `page_cursor()` off the last row of each
/// page and passes it back to the next [`FetchPageFn`] call so the query
/// can use `WHERE id > $cursor` instead of re-scanning from the start.
pub trait PageCursor {
	fn page_cursor(&self) -> Uuid;
}

impl PageCursor for entity::game::Model {
	fn page_cursor(&self) -> Uuid {
		self.id
	}
}

impl PageCursor for entity::company::Model {
	fn page_cursor(&self) -> Uuid {
		self.id
	}
}

impl PageCursor for entity::platform::Model {
	fn page_cursor(&self) -> Uuid {
		self.id
	}
}

/// Function pointer signature for the per-page fetch callback used by
/// [`drive_match_pipeline`]. Takes a `cursor` of the last id from the previous
/// page (or `None` for the first call) so the underlying query can skip rows
/// that have already been processed.
pub type FetchPageFn<M> = fn(
	MetadataProviderEnum,
	u64,
	Option<Uuid>,
	DbConn,
) -> BoxFuture<'static, anyhow::Result<Option<Vec<M>>>>;

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
	matched_year: Option<i16>,
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
		.matched_year(matched_year)
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
		FailedMatchReasonEnum::Ambiguous => "ambiguous",
		FailedMatchReasonEnum::TooManyFiles => "too_many_files",
	}
}

/// Drive a "page > chunk > spawn > join" matching loop. `fetch_fn` returns
/// the next page of unmatched entities, or `None` when drained. `match_fn`
/// runs per entity on its own task with a clone of `client` and `db_conn`.
/// Per-entity errors are logged with the `label` prefix and do not abort
/// the loop.
///
/// Uses keyset pagination via [`PageCursor`]: the last id of each page seeds
/// the next `fetch_fn` call so the query skips already-processed rows in
/// O(log N) rather than re-scanning from the smallest id every iteration.
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
	M: Clone + PageCursor + Send + 'static,
	C: Send + Sync + 'static,
{
	let mut cursor: Option<Uuid> = None;
	while let Some(page) = fetch_fn(provider, DEFAULT_PAGE_SIZE, cursor, db_conn.clone()).await? {
		let next_cursor = page.last().map(PageCursor::page_cursor);
		for chunk in page.chunks(chunk_size) {
			let mut handles = Vec::with_capacity(chunk.len());
			for entity in chunk.iter().cloned() {
				let client = client.clone();
				let conn = db_conn.clone();
				handles.push(tokio::spawn(match_fn(entity, client, conn)));
			}
			for handle in handles {
				if let Err(e) = handle.await? {
					error!("Error while matching {label} to provider: {e:#}");
				}
			}
		}
		cursor = match next_cursor {
			Some(c) => Some(c),
			None => break,
		};
	}
	Ok(())
}

/// Drive the clone-of-game propagation pattern shared by every provider's
/// `match_clone_of_game_to_*` entry point. Looks up the game's parent: if the
/// parent already has a matching mapping for this provider, propagates it
/// down as `ViaParent`. Otherwise runs the primary `match_fn` on the child
/// and, if the child becomes matched, propagates back up as `ViaChild`.
pub async fn drive_clone_propagation<C>(
	game: entity::game::Model,
	client: Arc<C>,
	db_conn: DbConn,
	primary_match_fn: MatchEntityFn<entity::game::Model, C>,
) -> anyhow::Result<()>
where
	C: MetadataProvider,
{
	let provider_label = client.provider_label();
	let provider_enum = client.provider_enum();
	let mut redis_conn = client.redis_conn().clone();

	let Some(parent_game) = find_game_parent(&game, &db_conn).await? else {
		return Ok(());
	};

	let parent_mapping = find_signature_metadata_mapping_by_platform_game_company_and_provider(
		None,
		Some(parent_game.id),
		None,
		provider_enum,
		&db_conn,
	)
	.await?;

	if let Some(mapping) = &parent_mapping
		&& matches!(
			mapping.match_type,
			MatchTypeEnum::Automatic | MatchTypeEnum::Manual
		) && let Some(provider_id) = mapping.provider_id.clone()
	{
		debug!(
			"Matched Game \"{}\" to {provider_label} Game ID {provider_id} (Via Parent)",
			&game.name
		);
		write_auto_match_success(
			provider_label,
			provider_enum,
			Target::Game(game.id),
			provider_id,
			AutomaticMatchReasonEnum::ViaParent,
			mapping.matched_name.clone(),
			mapping.matched_year,
			&db_conn,
			&mut redis_conn,
		)
		.await?;
		return Ok(());
	}

	primary_match_fn(game.clone(), client.clone(), db_conn.clone()).await?;

	let child_mapping = find_signature_metadata_mapping_by_platform_game_company_and_provider(
		None,
		Some(game.id),
		None,
		provider_enum,
		&db_conn,
	)
	.await?;

	if let Some(mapping) = child_mapping
		&& matches!(
			mapping.match_type,
			MatchTypeEnum::Automatic | MatchTypeEnum::Manual
		) && let Some(provider_id) = mapping.provider_id
	{
		debug!("Propagating {provider_label} match from clone to parent game (Via Child)");
		write_auto_match_success(
			provider_label,
			provider_enum,
			Target::Game(parent_game.id),
			provider_id,
			AutomaticMatchReasonEnum::ViaChild,
			mapping.matched_name,
			mapping.matched_year,
			&db_conn,
			&mut redis_conn,
		)
		.await?;
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
	let provider_label = provider_label_for(provider);
	let mut cursor: Option<Uuid> = None;
	while let Some(page) = crate::db::game::get_failed_games_for_cross_pass_with_limit(
		provider,
		DEFAULT_PAGE_SIZE,
		cursor,
		db_conn.clone(),
	)
	.await?
	{
		let next_cursor = page.last().map(PageCursor::page_cursor);
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
						return Ok::<&'static str, anyhow::Error>("no_siblings");
					}
					let names: Vec<String> = siblings.into_iter().map(|(_, n)| n).collect();
					match_fn(game, names, client, conn).await?;
					Ok::<&'static str, anyhow::Error>("attempted")
				}));
			}
			for handle in handles {
				match handle.await? {
					Ok(outcome) => {
						crate::metrics::record_cross_match_attempt(provider_label, outcome);
					}
					Err(e) => {
						error!("Error while cross-matching {label} to provider: {e:#}");
						crate::metrics::record_cross_match_attempt(provider_label, "error");
					}
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
		cursor = match next_cursor {
			Some(c) => Some(c),
			None => break,
		};
	}
	Ok(())
}

/// Stable string label for a provider, used as a metric label value. Must
/// stay in sync with each `MetadataProvider::provider_label` impl.
fn provider_label_for(provider: MetadataProviderEnum) -> &'static str {
	match provider {
		MetadataProviderEnum::Igdb => "igdb",
		MetadataProviderEnum::Steamgriddb => "steamgriddb",
		MetadataProviderEnum::Screenscraper => "screenscraper",
		MetadataProviderEnum::Mobygames => "mobygames",
		MetadataProviderEnum::Launchbox => "launchbox",
		MetadataProviderEnum::EmuReady => "emuready",
		MetadataProviderEnum::OpenVGDB => "openvgdb",
		MetadataProviderEnum::RetroAchievements => "retroachievements",
		MetadataProviderEnum::TheGamesDB => "thegamesdb",
	}
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

	/// Multiplexed Redis handle used by per-entity match fns for cache busts.
	/// Every provider participates in the identify cache contract, so the
	/// dependency is declared here rather than as an inherent method on each
	/// client.
	fn redis_conn(&self) -> &redis::aio::MultiplexedConnection;

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

	/// Providers without a `match_via_sibling_names` implementation should
	/// override this to `false` so the cross-match wave skips the EXISTS
	/// pre-check round trip entirely.
	fn supports_cross_match(&self) -> bool {
		true
	}

	/// Cheap `EXISTS` probe used by the parallel orchestrator to decide
	/// whether to spawn a primary match task for this provider.
	async fn has_outstanding_match_work(&self, db_conn: &DbConn) -> anyhow::Result<bool> {
		has_outstanding_match_work_for_provider(self.provider_enum(), db_conn).await
	}

	/// Cheap `EXISTS` probe used by the parallel orchestrator to decide
	/// whether to spawn a cross-match task for this provider.
	async fn has_outstanding_cross_match_work(&self, db_conn: &DbConn) -> anyhow::Result<bool> {
		if !self.supports_cross_match() {
			return Ok(false);
		}
		has_outstanding_cross_match_work_for_provider(self.provider_enum(), db_conn).await
	}
}

pub type ProviderRegistry = Vec<Arc<dyn MetadataProvider>>;

#[derive(Clone, Copy)]
enum Phase {
	Primary,
	CrossMatch,
}

/// Run the per-provider `EXISTS` probes in parallel and return only the
/// providers that have something to do this cycle. A probe error is treated
/// as "skip this provider this cycle"; the next cycle picks it back up.
async fn filter_outstanding(
	registry: &ProviderRegistry,
	db_conn: &DbConn,
	phase: Phase,
) -> Vec<Arc<dyn MetadataProvider>> {
	let mut checks: JoinSet<(usize, &'static str, anyhow::Result<bool>)> = JoinSet::new();
	for (idx, provider) in registry.iter().enumerate() {
		let provider = provider.clone();
		let db_conn = db_conn.clone();
		checks.spawn(async move {
			let label = provider.provider_label();
			let res = match phase {
				Phase::Primary => provider.has_outstanding_match_work(&db_conn).await,
				Phase::CrossMatch => provider.has_outstanding_cross_match_work(&db_conn).await,
			};
			(idx, label, res)
		});
	}

	let mut keep = vec![false; registry.len()];
	while let Some(joined) = checks.join_next().await {
		match joined {
			Ok((idx, _, Ok(true))) => {
				keep[idx] = true;
			}
			Ok((_, _, Ok(false))) => {}
			Ok((_, label, Err(e))) => {
				error!("Outstanding-work check for '{label}' failed: {e:#}; skipping this cycle");
			}
			Err(join_err) => {
				error!("Outstanding-work check task panicked: {join_err:?}");
			}
		}
	}

	registry
		.iter()
		.enumerate()
		.filter_map(|(idx, p)| if keep[idx] { Some(p.clone()) } else { None })
		.collect()
}

fn join_labels(providers: &[Arc<dyn MetadataProvider>]) -> String {
	let mut labels: Vec<&'static str> = providers.iter().map(|p| p.provider_label()).collect();
	labels.sort_unstable();
	labels.join(", ")
}

async fn run_primary_wave(registry: &ProviderRegistry, db_conn: &DbConn) {
	let outstanding = filter_outstanding(registry, db_conn, Phase::Primary).await;
	if outstanding.is_empty() {
		info!("No providers have outstanding match work this cycle");
		return;
	}
	let labels = join_labels(&outstanding);
	info!("Starting match cycle for providers {labels}");

	let mut set: JoinSet<()> = JoinSet::new();
	for provider in outstanding {
		let db_conn = db_conn.clone();
		set.spawn(async move {
			let label = provider.provider_label();
			let started = Instant::now();
			let result = match provider.clone().match_db(&db_conn).await {
				Ok(()) => {
					info!("Finished match cycle for provider '{label}'");
					"success"
				}
				Err(e) => {
					error!("Provider '{label}' match cycle failed: {e:#}");
					"failure"
				}
			};
			record_background_job(
				&format!("{label}_match"),
				result,
				started.elapsed().as_secs_f64(),
			);
		});
	}
	while let Some(res) = set.join_next().await {
		if let Err(join_err) = res {
			error!("Provider match task panicked or was cancelled: {join_err:?}");
		}
	}
}

async fn run_cross_match_wave(registry: &ProviderRegistry, db_conn: &DbConn) {
	let outstanding = filter_outstanding(registry, db_conn, Phase::CrossMatch).await;
	if outstanding.is_empty() {
		info!("No providers have outstanding cross-provider match work this cycle");
		return;
	}
	let labels = join_labels(&outstanding);
	info!("Starting cross-provider name retry pass for providers {labels}");

	let mut set: JoinSet<()> = JoinSet::new();
	for provider in outstanding {
		let db_conn = db_conn.clone();
		set.spawn(async move {
			let label = provider.provider_label();
			let started = Instant::now();
			let result = match provider.clone().match_via_sibling_names(&db_conn).await {
				Ok(()) => {
					info!("Finished cross-provider name retry pass for provider '{label}'");
					"success"
				}
				Err(e) => {
					error!("Provider '{label}' cross-provider name pass failed: {e:#}");
					"failure"
				}
			};
			record_background_job(
				&format!("{label}_cross_match"),
				result,
				started.elapsed().as_secs_f64(),
			);
		});
	}
	while let Some(res) = set.join_next().await {
		if let Err(join_err) = res {
			error!("Cross-provider match task panicked or was cancelled: {join_err:?}");
		}
	}
}

/// Per-provider failures are logged and recorded as `{label}_match` failure
/// metrics; they do not stop other providers. Wave 2 (cross-provider) runs
/// only after every provider's Wave 1 task has finished so each provider's
/// cross-pass sees the full set of sibling `matched_name` values.
pub async fn match_db_to_all_providers(
	registry: &ProviderRegistry,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	let aggregate_started = Instant::now();
	run_primary_wave(registry, db_conn).await;
	run_cross_match_wave(registry, db_conn).await;
	record_background_job(
		"provider_match_all",
		"success",
		aggregate_started.elapsed().as_secs_f64(),
	);
	Ok(())
}
