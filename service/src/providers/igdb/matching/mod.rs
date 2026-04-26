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
use log::info;
use sea_orm::DbConn;
use sea_orm::prelude::Uuid;
use std::sync::Arc;

use platform::match_platforms_to_igdb;

pub(crate) const PAGE_SIZE: u64 = 100;
pub(crate) const IGDB_CHUNK_SIZE: usize = 4;

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
