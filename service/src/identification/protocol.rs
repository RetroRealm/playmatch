//! Cache-coherence protocol for the identify pipeline. Lifted out of
//! [`super::identify_game`] so the per-match-type bookkeeping and the
//! `Cached(None)` vs `NonCached(None)` decision can be unit-tested without
//! a live Redis or DB.

use crate::cache::CacheStatus;
use crate::cache::CacheStatus::{Cached, NonCached};
use crate::identification::cache::IdentifyEntry;
use crate::model::GameMatchType;
use std::ops::ControlFlow;

/// Running-tally state machine for the identify dispatch loop. Caller feeds
/// each per-match-type cache outcome through [`Self::observe`] and short
/// circuits when the result is `Break`; if every attempt completes without
/// a hit, [`Self::finalize`] collapses the tally into a final
/// `Cached(None)` / `NonCached(None)`.
///
/// `expected_count` is the number of content-key fields (sha256, sha1, md5,
/// crc) the caller will attempt. The filename and size attempt is excluded so
/// that a `Cached(None)` verdict requires every content key to have been a
/// cached miss, not merely the cheaper filename and size lookup.
pub struct IdentifyAggregator {
	expected_count: usize,
	cached_but_empty: usize,
}

impl IdentifyAggregator {
	pub fn new(expected_count: usize) -> Self {
		Self {
			expected_count,
			cached_but_empty: 0,
		}
	}

	/// Fold one per-match-type cache outcome into the running tally.
	pub fn observe(
		&mut self,
		match_type: GameMatchType,
		outcome: CacheStatus<Option<IdentifyEntry>>,
	) -> ControlFlow<CacheStatus<Option<(GameMatchType, IdentifyEntry)>>> {
		match outcome {
			Cached(Some(entry)) => ControlFlow::Break(Cached(Some((match_type, entry)))),
			NonCached(Some(entry)) => ControlFlow::Break(NonCached(Some((match_type, entry)))),
			Cached(None) => {
				self.cached_but_empty += 1;
				ControlFlow::Continue(())
			}
			NonCached(None) => ControlFlow::Continue(()),
		}
	}

	/// Collapse the running tally into a final cache outcome.
	///
	/// Returns `Cached(None)` only when every content-key attempt produced
	/// `Cached(None)`.
	pub fn finalize(self) -> CacheStatus<Option<(GameMatchType, IdentifyEntry)>> {
		if self.cached_but_empty == self.expected_count && self.cached_but_empty != 0 {
			Cached(None)
		} else {
			NonCached(None)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use entity::game;
	use sea_orm::prelude::Uuid;

	fn fake_entry() -> IdentifyEntry {
		IdentifyEntry {
			game: game::Model {
				id: Uuid::nil(),
				dat_file_import_id: Uuid::nil(),
				signature_group_internal_id: None,
				name: String::new(),
				description: None,
				categories: None,
				clone_of: None,
				created_at: chrono::Utc::now().fixed_offset(),
				updated_at: chrono::Utc::now().fixed_offset(),
				signature_group_internal_clone_of_id: None,
				last_seen_dat_file_import_id: None,
				is_current: true,
				content_anchor_id: None,
			},
			metadata_mappings: Vec::new(),
		}
	}

	#[test]
	fn first_cached_hit_short_circuits() {
		let mut agg = IdentifyAggregator::new(3);
		let flow = agg.observe(GameMatchType::SHA256, Cached(Some(fake_entry())));
		assert!(matches!(
			flow,
			ControlFlow::Break(Cached(Some((GameMatchType::SHA256, _))))
		));
	}

	#[test]
	fn first_noncached_hit_short_circuits() {
		let mut agg = IdentifyAggregator::new(3);
		let flow = agg.observe(GameMatchType::SHA1, NonCached(Some(fake_entry())));
		assert!(matches!(
			flow,
			ControlFlow::Break(NonCached(Some((GameMatchType::SHA1, _))))
		));
	}

	#[test]
	fn all_three_hashes_cached_empty_yields_cached_none() {
		let mut agg = IdentifyAggregator::new(3);
		for mt in [
			GameMatchType::SHA256,
			GameMatchType::SHA1,
			GameMatchType::MD5,
		] {
			assert!(matches!(
				agg.observe(mt, Cached(None)),
				ControlFlow::Continue(())
			));
		}
		assert!(matches!(agg.finalize(), Cached(None)));
	}

	#[test]
	fn one_hash_cached_empty_one_noncached_empty_yields_noncached_none() {
		let mut agg = IdentifyAggregator::new(2);
		assert!(matches!(
			agg.observe(GameMatchType::SHA256, Cached(None)),
			ControlFlow::Continue(())
		));
		assert!(matches!(
			agg.observe(GameMatchType::SHA1, NonCached(None)),
			ControlFlow::Continue(())
		));
		assert!(matches!(agg.finalize(), NonCached(None)));
	}

	/// Pins a known wrinkle: when every hash is cached empty AND the
	/// filename+size attempt is also cached empty, `cached_but_empty`
	/// (3 hashes + 1 filename = 4) no longer equals `expected_count`
	/// (3 hashes), so the result is `NonCached(None)` even though every
	/// attempt was cached.
	#[test]
	fn filename_size_cached_empty_breaks_cached_none_collapse() {
		let mut agg = IdentifyAggregator::new(3);
		for mt in [
			GameMatchType::SHA256,
			GameMatchType::SHA1,
			GameMatchType::MD5,
		] {
			let _ = agg.observe(mt, Cached(None));
		}
		let _ = agg.observe(GameMatchType::FileNameAndSize, Cached(None));
		assert!(matches!(agg.finalize(), NonCached(None)));
	}

	/// Pins a second wrinkle: when only filename+size is attempted (no
	/// hashes) and it returns Cached(None), the `expected_count != 0`
	/// guard in `finalize` forces `NonCached(None)`.
	#[test]
	fn only_filename_size_cached_empty_yields_noncached_none() {
		let mut agg = IdentifyAggregator::new(0);
		let _ = agg.observe(GameMatchType::FileNameAndSize, Cached(None));
		assert!(matches!(agg.finalize(), NonCached(None)));
	}

	#[test]
	fn empty_search_yields_noncached_none() {
		let agg = IdentifyAggregator::new(0);
		assert!(matches!(agg.finalize(), NonCached(None)));
	}

	#[test]
	fn partial_hash_count_with_all_cached_empty_collapses() {
		let mut agg = IdentifyAggregator::new(2);
		let _ = agg.observe(GameMatchType::SHA256, Cached(None));
		let _ = agg.observe(GameMatchType::MD5, Cached(None));
		assert!(matches!(agg.finalize(), Cached(None)));
	}
}
