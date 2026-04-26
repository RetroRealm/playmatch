mod company;
mod game;
mod platform;

use self::game::match_games_to_igdb;
use crate::db::signature_metadata_mapping::{
	SignatureMetadataMappingInputBuilder, create_or_update_signature_metadata_mapping,
};
use crate::providers::igdb::IgdbClient;
use company::match_companies_to_igdb;
use entity::sea_orm_active_enums::{
	AutomaticMatchReasonEnum, FailedMatchReasonEnum, MatchTypeEnum, MetadataProviderEnum,
};
use futures_util::future::BoxFuture;
use log::{error, info};
use sea_orm::DbConn;
use sea_orm::prelude::Uuid;
use std::sync::Arc;

use platform::match_platforms_to_igdb;

pub(crate) const PAGE_SIZE: u64 = 100;
pub(crate) const IGDB_CHUNK_SIZE: usize = 4;

/// Function pointer signature for the per-page fetch callback used by
/// [`drive_match_pipeline`]. Returns the next page of unmatched entities of
/// type `M` for the given provider, or `None` when the queue is drained.
pub(crate) type FetchPageFn<M> =
	fn(MetadataProviderEnum, u64, DbConn) -> BoxFuture<'static, anyhow::Result<Option<Vec<M>>>>;

/// Function pointer signature for the per-entity match callback used by
/// [`drive_match_pipeline`]. Receives one entity, an `Arc` clone of the
/// provider client, and an owned `DbConn`.
pub(crate) type MatchEntityFn<M, C> =
	fn(M, Arc<C>, DbConn) -> BoxFuture<'static, anyhow::Result<()>>;

pub async fn match_db_to_igdb_entities(
	igdb_client: Arc<IgdbClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	match_companies_to_igdb(igdb_client.clone(), db_conn).await?;
	info!("Finished matching companies to IGDB");

	match_platforms_to_igdb(igdb_client.clone(), db_conn).await?;
	info!("Finished matching platforms to IGDB");

	match_games_to_igdb(igdb_client.clone(), db_conn).await?;
	info!("Finished matching games to IGDB");
	Ok(())
}

/// Drive a "page > chunk > spawn > join" matching loop. `fetch_fn` returns the next
/// page of unmatched entities (or `None` when nothing is left). `match_fn` is invoked
/// per entity, on its own task, with a clone of `client` and `db_conn`. Per-entity
/// errors are logged with the `label` prefix and do not abort the loop.
pub(crate) async fn drive_match_pipeline<M, C>(
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
	while let Some(page) = fetch_fn(provider, PAGE_SIZE, db_conn.clone()).await? {
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

/// Identifies which entity table a match write targets. Carries the FK value
/// so the writer helpers can populate the matching column on the
/// `signature_metadata_mapping` row.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Target {
	Company(Uuid),
	Platform(Uuid),
	Game(Uuid),
}

impl Target {
	pub(crate) fn entity_label(self) -> &'static str {
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

/// Persist a successful automatic match and increment the matching metric.
pub(crate) async fn write_auto_match_success(
	provider_label: &'static str,
	provider_enum: MetadataProviderEnum,
	target: Target,
	provider_id: String,
	reason: AutomaticMatchReasonEnum,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	let mut builder = SignatureMetadataMappingInputBuilder::default();
	target.apply(&mut builder);
	let input = builder
		.provider(provider_enum)
		.provider_id(Some(provider_id))
		.match_type(MatchTypeEnum::Automatic)
		.automatic_match_reason(Some(reason))
		.build()?;
	create_or_update_signature_metadata_mapping(input, db_conn).await?;
	crate::metrics::record_metadata_auto_match(
		provider_label,
		target.entity_label(),
		"matched",
		automatic_reason_label(reason),
	);
	Ok(())
}

/// Persist a failed automatic match and increment the matching metric.
pub(crate) async fn write_auto_match_failed(
	provider_label: &'static str,
	provider_enum: MetadataProviderEnum,
	target: Target,
	reason: FailedMatchReasonEnum,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	let mut builder = SignatureMetadataMappingInputBuilder::default();
	target.apply(&mut builder);
	let input = builder
		.provider(provider_enum)
		.match_type(MatchTypeEnum::Failed)
		.failed_match_reason(Some(reason))
		.build()?;
	create_or_update_signature_metadata_mapping(input, db_conn).await?;
	crate::metrics::record_metadata_auto_match(
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
