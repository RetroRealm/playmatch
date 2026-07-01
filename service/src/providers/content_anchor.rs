use crate::db::game::find_display_priorities_for_games;
use crate::db::game_file::{find_games_by_content_anchor, find_multi_member_anchor_ids};
use crate::db::signature_metadata_mapping::find_signature_metadata_mappings_by_game_ids;
use crate::providers::{ProviderRegistry, Target, provider_label_for, write_auto_match_success};
use entity::game;
use entity::sea_orm_active_enums::{AutomaticMatchReasonEnum, MatchTypeEnum, MetadataProviderEnum};
use entity::signature_metadata_mapping::Model as MappingModel;
use log::error;
use sea_orm::prelude::Uuid;
use sea_orm::{DbConn, Iterable};
use std::collections::HashMap;
use tokio::task::JoinHandle;

const CONTENT_ANCHOR_CHUNK_SIZE: usize = 16;

/// Strength of a mapping for survivorship: Manual outranks Automatic, which
/// outranks Failed/None. Manual always wins and is never downgraded.
fn match_type_strength(match_type: MatchTypeEnum) -> u8 {
	match match_type {
		MatchTypeEnum::Manual => 2,
		MatchTypeEnum::Automatic => 1,
		MatchTypeEnum::Failed | MatchTypeEnum::None => 0,
	}
}

struct Survivor<'a> {
	game_id: Uuid,
	mapping: &'a MappingModel,
}

/// Deterministic survivor for one `(anchor, provider)` class:
/// `match_type` strength first, then `signature_group.display_priority`, then
/// the lowest game UUID. Only mappings carrying a usable `provider_id` and a
/// non-none match type are candidates. Returns `None` when no member has a
/// usable mapping for the provider.
fn pick_survivor<'a>(
	provider: MetadataProviderEnum,
	mappings_by_game: &'a HashMap<Uuid, Vec<MappingModel>>,
	display_priorities: &HashMap<Uuid, i16>,
) -> Option<Survivor<'a>> {
	let mut best: Option<Survivor<'a>> = None;

	for (game_id, mappings) in mappings_by_game {
		let Some(mapping) = mappings.iter().find(|m| m.provider == provider) else {
			continue;
		};
		if mapping.provider_id.is_none()
			|| matches!(
				mapping.match_type,
				MatchTypeEnum::Failed | MatchTypeEnum::None
			) {
			continue;
		}

		let candidate = Survivor {
			game_id: *game_id,
			mapping,
		};

		match &best {
			None => best = Some(candidate),
			Some(current) => {
				if survivor_is_better(&candidate, current, display_priorities) {
					best = Some(candidate);
				}
			}
		}
	}

	best
}

fn survivor_is_better(
	candidate: &Survivor,
	current: &Survivor,
	display_priorities: &HashMap<Uuid, i16>,
) -> bool {
	let cand_strength = match_type_strength(candidate.mapping.match_type);
	let cur_strength = match_type_strength(current.mapping.match_type);
	if cand_strength != cur_strength {
		return cand_strength > cur_strength;
	}

	let cand_priority = display_priorities
		.get(&candidate.game_id)
		.copied()
		.unwrap_or(i16::MAX);
	let cur_priority = display_priorities
		.get(&current.game_id)
		.copied()
		.unwrap_or(i16::MAX);
	if cand_priority != cur_priority {
		return cand_priority < cur_priority;
	}

	candidate.game_id < current.game_id
}

/// Write `survivor`'s provider mapping onto `target_game` as a `ViaContentHash`
/// automatic match, unless the target already holds a Manual mapping for that
/// provider (the hard never-downgrade floor) or already agrees with the
/// survivor. Reuses [`write_auto_match_success`] so the write busts the cache.
#[allow(clippy::too_many_arguments)]
async fn survive_and_write(
	provider: MetadataProviderEnum,
	provider_id: String,
	matched_name: Option<String>,
	matched_year: Option<i16>,
	target_game_id: Uuid,
	target_mapping: Option<&MappingModel>,
	db_conn: &DbConn,
	redis_conn: &mut redis::aio::MultiplexedConnection,
) -> anyhow::Result<bool> {
	if let Some(existing) = target_mapping {
		if existing.match_type == MatchTypeEnum::Manual {
			return Ok(false);
		}
		if existing.provider_id.as_deref() == Some(provider_id.as_str())
			&& existing.match_type == MatchTypeEnum::Automatic
		{
			return Ok(false);
		}
	}

	write_auto_match_success(
		provider_label_for(provider),
		provider,
		Target::Game(target_game_id),
		provider_id,
		AutomaticMatchReasonEnum::ViaContentHash,
		matched_name,
		matched_year,
		db_conn,
		redis_conn,
	)
	.await?;

	Ok(true)
}

/// Recompute the survivor per metadata provider for one anchor and rewrite only
/// disagreeing members. Converged classes write nothing, so the pass is
/// idempotent. Both-Manual-with-different-provider_id pairs are left untouched.
async fn reconcile_anchor(
	anchor_id: Uuid,
	db_conn: &DbConn,
	redis_conn: &mut redis::aio::MultiplexedConnection,
) -> anyhow::Result<()> {
	let games = find_games_by_content_anchor(anchor_id, db_conn).await?;
	if games.len() < 2 {
		return Ok(());
	}

	let game_ids: Vec<Uuid> = games.iter().map(|g| g.id).collect();
	let mappings_by_game = find_signature_metadata_mappings_by_game_ids(&game_ids, db_conn).await?;
	let display_priorities = find_display_priorities_for_games(&game_ids, db_conn).await?;

	for provider in MetadataProviderEnum::iter() {
		let Some(survivor) = pick_survivor(provider, &mappings_by_game, &display_priorities) else {
			continue;
		};
		let Some(provider_id) = survivor.mapping.provider_id.clone() else {
			continue;
		};
		let matched_name = survivor.mapping.matched_name.clone();
		let matched_year = survivor.mapping.matched_year;

		for game_id in &game_ids {
			if *game_id == survivor.game_id {
				continue;
			}
			let target_mapping = mappings_by_game
				.get(game_id)
				.and_then(|m| m.iter().find(|m| m.provider == provider));

			if let Err(e) = survive_and_write(
				provider,
				provider_id.clone(),
				matched_name.clone(),
				matched_year,
				*game_id,
				target_mapping,
				db_conn,
				redis_conn,
			)
			.await
			{
				error!(
					"content-anchor reconcile failed for game {game_id} provider {provider:?}: {e:#}"
				);
			}
		}
	}

	Ok(())
}

/// Third wave of [`crate::providers::match_db_to_all_providers`]: after the
/// primary and cross-provider waves have fully populated mappings, converge
/// every multi-member content anchor toward its deterministic survivor per
/// metadata provider. Chunked like
/// [`crate::matching::clone::populate_clone_of_id`].
pub async fn run_content_anchor_reconcile_wave(
	registry: &ProviderRegistry,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	let Some(provider) = registry.first() else {
		return Ok(());
	};
	let redis_conn = provider.redis_conn().clone();

	let anchor_ids = find_multi_member_anchor_ids(db_conn).await?;
	for chunk in anchor_ids.chunks(CONTENT_ANCHOR_CHUNK_SIZE) {
		let mut futures: Vec<JoinHandle<()>> = vec![];

		for anchor_id in chunk.iter().copied() {
			let db_conn = db_conn.clone();
			let mut redis_conn = redis_conn.clone();
			futures.push(tokio::spawn(async move {
				if let Err(e) = reconcile_anchor(anchor_id, &db_conn, &mut redis_conn).await {
					error!("content-anchor reconcile failed for anchor {anchor_id}: {e:#}");
				}
			}));
		}

		for future in futures {
			if let Err(e) = future.await {
				error!("content-anchor reconcile task panicked: {e:?}");
			}
		}
	}

	Ok(())
}

/// Seed mappings onto a freshly anchored `game` from an already-mapped sibling
/// in the same anchor, but only for providers where the target has no existing
/// non-none mapping. Never clobbers a Manual or a fresh upstream match.
pub async fn seed_mappings_from_sibling(
	game: &game::Model,
	anchor_id: Uuid,
	db_conn: &DbConn,
	redis_conn: &mut redis::aio::MultiplexedConnection,
) -> anyhow::Result<()> {
	let siblings = find_games_by_content_anchor(anchor_id, db_conn).await?;
	let sibling_ids: Vec<Uuid> = siblings
		.iter()
		.map(|g| g.id)
		.filter(|id| *id != game.id)
		.collect();
	if sibling_ids.is_empty() {
		return Ok(());
	}

	let mut lookup_ids = sibling_ids.clone();
	lookup_ids.push(game.id);
	let mappings_by_game =
		find_signature_metadata_mappings_by_game_ids(&lookup_ids, db_conn).await?;
	let display_priorities = find_display_priorities_for_games(&lookup_ids, db_conn).await?;

	let target_mappings = mappings_by_game.get(&game.id);

	for provider in MetadataProviderEnum::iter() {
		let already_mapped = target_mappings
			.map(|m| {
				m.iter().any(|mapping| {
					mapping.provider == provider
						&& !matches!(
							mapping.match_type,
							MatchTypeEnum::Failed | MatchTypeEnum::None
						)
				})
			})
			.unwrap_or(false);
		if already_mapped {
			continue;
		}

		let Some(survivor) = pick_survivor(provider, &mappings_by_game, &display_priorities) else {
			continue;
		};
		if survivor.game_id == game.id {
			continue;
		}
		let Some(provider_id) = survivor.mapping.provider_id.clone() else {
			continue;
		};

		if let Err(e) = survive_and_write(
			provider,
			provider_id,
			survivor.mapping.matched_name.clone(),
			survivor.mapping.matched_year,
			game.id,
			target_mappings.and_then(|m| m.iter().find(|m| m.provider == provider)),
			db_conn,
			redis_conn,
		)
		.await
		{
			error!(
				"content-anchor seed failed for game {} provider {provider:?}: {e:#}",
				game.id
			);
		}
	}

	Ok(())
}
