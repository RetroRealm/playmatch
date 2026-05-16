use crate::matching::name_parse::{ParsedName, Variant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum YearMatch {
	Unknown,
	OffByOne,
	Exact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RegionMatch {
	Unknown,
	Other,
	Default,
	Exact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VariantMatch {
	Mismatch,
	Compatible,
	BothNone,
	Same,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CandidateScore {
	pub year_match: YearMatch,
	pub region_match: RegionMatch,
	pub variant_match: VariantMatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateGate {
	Reject,
	Pass(CandidateScore),
}

/// Combined gate + scorer used by the scored ladder. Returns `Reject` for
/// hard mismatches (year off by >=2, candidate platform list excludes the
/// DAT platform, hack/pirate/unlicensed DAT against a candidate that does
/// not carry the same marker). Otherwise returns a `CandidateScore` whose
/// `Ord` ranks year first, then region, then variant agreement.
#[allow(clippy::too_many_arguments)]
pub fn gate_and_score(
	parsed_dat: &ParsedName,
	parsed_cand: Option<&ParsedName>,
	candidate_year: Option<u16>,
	candidate_region_codes: &[&str],
	provider_default_regions: &[&str],
	candidate_platforms: Option<&[i64]>,
	dat_platform_id: Option<i64>,
	dat_region_codes: &[&str],
) -> CandidateGate {
	if let Some(platforms) = candidate_platforms
		&& !platforms.is_empty()
		&& let Some(dat_pid) = dat_platform_id
		&& !platforms.contains(&dat_pid)
	{
		return CandidateGate::Reject;
	}

	if matches!(
		parsed_dat.variant,
		Some(Variant::Hack) | Some(Variant::Pirate) | Some(Variant::Unlicensed)
	) {
		let cand_variant = parsed_cand.and_then(|p| p.variant);
		if cand_variant != parsed_dat.variant {
			return CandidateGate::Reject;
		}
	}

	let year_match = match (parsed_dat.year, candidate_year) {
		(Some(dat_year), Some(cand_year)) => {
			let delta = dat_year.abs_diff(cand_year);
			if delta >= 2 {
				return CandidateGate::Reject;
			} else if delta == 0 {
				YearMatch::Exact
			} else {
				YearMatch::OffByOne
			}
		}
		_ => YearMatch::Unknown,
	};

	let region_match = score_region(
		candidate_region_codes,
		dat_region_codes,
		provider_default_regions,
	);

	let variant_match = score_variant(parsed_dat.variant, parsed_cand.and_then(|p| p.variant));

	CandidateGate::Pass(CandidateScore {
		year_match,
		region_match,
		variant_match,
	})
}

fn score_region(
	candidate_codes: &[&str],
	dat_codes: &[&str],
	provider_default_codes: &[&str],
) -> RegionMatch {
	if candidate_codes.is_empty() && dat_codes.is_empty() {
		return RegionMatch::Unknown;
	}
	if !dat_codes.is_empty()
		&& candidate_codes
			.iter()
			.any(|c| dat_codes.iter().any(|d| d.eq_ignore_ascii_case(c)))
	{
		return RegionMatch::Exact;
	}
	if candidate_codes.iter().any(|c| {
		provider_default_codes
			.iter()
			.any(|d| d.eq_ignore_ascii_case(c))
	}) {
		return RegionMatch::Default;
	}
	if !candidate_codes.is_empty() {
		return RegionMatch::Other;
	}
	RegionMatch::Unknown
}

fn score_variant(dat: Option<Variant>, cand: Option<Variant>) -> VariantMatch {
	match (dat, cand) {
		(None, None) => VariantMatch::BothNone,
		(Some(d), Some(c)) if d == c => VariantMatch::Same,
		(Some(Variant::Proto), None)
		| (Some(Variant::Beta), None)
		| (Some(Variant::Demo), None)
		| (Some(Variant::Sample), None) => VariantMatch::Compatible,
		_ => VariantMatch::Mismatch,
	}
}

#[derive(Debug, Clone, Copy)]
pub enum Selection<'a, T> {
	Best(&'a T),
	Ambiguous,
	None,
}

/// Stable label for a rung outcome, used by the per-rung match metric.
pub fn rung_outcome_label<T>(sel: &Selection<'_, T>) -> &'static str {
	match sel {
		Selection::Best(_) => "hit",
		Selection::Ambiguous => "ambiguous",
		Selection::None => "miss",
	}
}

/// Records the outcome of a `pick_best` call as a per-rung metric and
/// returns the `Selection` unchanged so call sites can continue to chain.
pub fn record_pick_best<'a, T>(
	provider: &str,
	rung: &str,
	sel: Selection<'a, T>,
) -> Selection<'a, T> {
	crate::metrics::record_match_rung(provider, rung, rung_outcome_label(&sel));
	sel
}

/// Returns the unique max-scored candidate. Ties at the top of the score
/// produce `Ambiguous`; empty input produces `None`.
pub fn pick_best<'a, T>(
	candidates: impl IntoIterator<Item = (&'a T, CandidateScore)>,
) -> Selection<'a, T> {
	let mut best: Option<(&T, CandidateScore)> = None;
	let mut tied = false;
	for (cand, score) in candidates {
		match best {
			None => {
				best = Some((cand, score));
				tied = false;
			}
			Some((_, b_score)) if score > b_score => {
				best = Some((cand, score));
				tied = false;
			}
			Some((_, b_score)) if score == b_score => {
				tied = true;
			}
			_ => {}
		}
	}
	match best {
		None => Selection::None,
		Some(_) if tied => Selection::Ambiguous,
		Some((c, _)) => Selection::Best(c),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::matching::name_parse::RegionTag;

	#[test]
	fn rung_outcome_label_maps_each_variant() {
		let item = "x";
		let best: Selection<'_, &str> = Selection::Best(&item);
		assert_eq!(rung_outcome_label(&best), "hit");
		let ambig: Selection<'_, &str> = Selection::Ambiguous;
		assert_eq!(rung_outcome_label(&ambig), "ambiguous");
		let none: Selection<'_, &str> = Selection::None;
		assert_eq!(rung_outcome_label(&none), "miss");
	}

	fn dat(year: Option<u16>) -> ParsedName {
		ParsedName {
			year,
			..ParsedName::default()
		}
	}

	fn dat_with_variant(v: Variant) -> ParsedName {
		ParsedName {
			variant: Some(v),
			..ParsedName::default()
		}
	}

	fn cand_with_variant(v: Option<Variant>) -> ParsedName {
		ParsedName {
			variant: v,
			..ParsedName::default()
		}
	}

	fn dat_with_region(r: RegionTag) -> ParsedName {
		ParsedName {
			regions: vec![r],
			..ParsedName::default()
		}
	}

	#[test]
	fn gate_no_signals_passes_with_unknown_score() {
		match gate_and_score(&dat(None), None, None, &[], &[], None, None, &[]) {
			CandidateGate::Pass(s) => {
				assert_eq!(s.year_match, YearMatch::Unknown);
				assert_eq!(s.region_match, RegionMatch::Unknown);
				assert_eq!(s.variant_match, VariantMatch::BothNone);
			}
			CandidateGate::Reject => panic!("expected pass"),
		}
	}

	#[test]
	fn gate_year_off_by_two_rejects() {
		assert!(matches!(
			gate_and_score(
				&dat(Some(2000)),
				None,
				Some(2002),
				&[],
				&[],
				None,
				None,
				&[]
			),
			CandidateGate::Reject
		));
	}

	#[test]
	fn gate_year_off_by_one_scores_below_exact() {
		let off = match gate_and_score(
			&dat(Some(2000)),
			None,
			Some(2001),
			&[],
			&[],
			None,
			None,
			&[],
		) {
			CandidateGate::Pass(s) => s,
			_ => panic!(),
		};
		let exact = match gate_and_score(
			&dat(Some(2000)),
			None,
			Some(2000),
			&[],
			&[],
			None,
			None,
			&[],
		) {
			CandidateGate::Pass(s) => s,
			_ => panic!(),
		};
		assert!(exact > off);
	}

	#[test]
	fn gate_platform_mismatch_rejects() {
		assert!(matches!(
			gate_and_score(
				&dat(None),
				None,
				None,
				&[],
				&[],
				Some(&[3, 11]),
				Some(7),
				&[]
			),
			CandidateGate::Reject
		));
	}

	#[test]
	fn gate_platform_match_passes() {
		assert!(matches!(
			gate_and_score(
				&dat(None),
				None,
				None,
				&[],
				&[],
				Some(&[3, 7, 11]),
				Some(7),
				&[]
			),
			CandidateGate::Pass(_)
		));
	}

	#[test]
	fn gate_hack_against_production_rejects() {
		let parsed = dat_with_variant(Variant::Hack);
		let cand = cand_with_variant(None);
		assert!(matches!(
			gate_and_score(&parsed, Some(&cand), None, &[], &[], None, None, &[]),
			CandidateGate::Reject
		));
	}

	#[test]
	fn gate_pirate_against_production_rejects() {
		let parsed = dat_with_variant(Variant::Pirate);
		let cand = cand_with_variant(None);
		assert!(matches!(
			gate_and_score(&parsed, Some(&cand), None, &[], &[], None, None, &[]),
			CandidateGate::Reject
		));
	}

	#[test]
	fn gate_unlicensed_against_production_rejects() {
		let parsed = dat_with_variant(Variant::Unlicensed);
		let cand = cand_with_variant(None);
		assert!(matches!(
			gate_and_score(&parsed, Some(&cand), None, &[], &[], None, None, &[]),
			CandidateGate::Reject
		));
	}

	#[test]
	fn gate_hack_against_hack_passes_with_same() {
		let parsed = dat_with_variant(Variant::Hack);
		let cand = cand_with_variant(Some(Variant::Hack));
		match gate_and_score(&parsed, Some(&cand), None, &[], &[], None, None, &[]) {
			CandidateGate::Pass(s) => assert_eq!(s.variant_match, VariantMatch::Same),
			_ => panic!(),
		}
	}

	#[test]
	fn gate_proto_against_production_passes_compatible() {
		let parsed = dat_with_variant(Variant::Proto);
		let cand = cand_with_variant(None);
		match gate_and_score(&parsed, Some(&cand), None, &[], &[], None, None, &[]) {
			CandidateGate::Pass(s) => assert_eq!(s.variant_match, VariantMatch::Compatible),
			_ => panic!("proto should match production with Compatible"),
		}
	}

	#[test]
	fn gate_beta_against_production_passes_compatible() {
		let parsed = dat_with_variant(Variant::Beta);
		let cand = cand_with_variant(None);
		match gate_and_score(&parsed, Some(&cand), None, &[], &[], None, None, &[]) {
			CandidateGate::Pass(s) => assert_eq!(s.variant_match, VariantMatch::Compatible),
			_ => panic!(),
		}
	}

	#[test]
	fn gate_both_none_variant() {
		match gate_and_score(
			&dat(None),
			Some(&ParsedName::default()),
			None,
			&[],
			&[],
			None,
			None,
			&[],
		) {
			CandidateGate::Pass(s) => assert_eq!(s.variant_match, VariantMatch::BothNone),
			_ => panic!(),
		}
	}

	#[test]
	fn gate_region_exact_beats_default_beats_other() {
		let parsed = dat_with_region(RegionTag::Japan);
		let dat_codes: Vec<&str> = parsed
			.regions
			.iter()
			.flat_map(|r| r.ss_codes().iter().copied())
			.collect();
		let exact = match gate_and_score(
			&parsed,
			Some(&ParsedName::default()),
			None,
			&["jp"],
			&["wor", "us", "eu"],
			None,
			None,
			&dat_codes,
		) {
			CandidateGate::Pass(s) => s,
			_ => panic!(),
		};
		let default = match gate_and_score(
			&parsed,
			Some(&ParsedName::default()),
			None,
			&["us"],
			&["wor", "us", "eu"],
			None,
			None,
			&dat_codes,
		) {
			CandidateGate::Pass(s) => s,
			_ => panic!(),
		};
		let other = match gate_and_score(
			&parsed,
			Some(&ParsedName::default()),
			None,
			&["br"],
			&["wor", "us", "eu"],
			None,
			None,
			&dat_codes,
		) {
			CandidateGate::Pass(s) => s,
			_ => panic!(),
		};
		assert!(exact.region_match > default.region_match);
		assert!(default.region_match > other.region_match);
	}

	#[test]
	fn pick_best_unique_max() {
		struct C(u32);
		let a = C(1);
		let b = C(2);
		let lo = CandidateScore {
			year_match: YearMatch::Unknown,
			region_match: RegionMatch::Unknown,
			variant_match: VariantMatch::Mismatch,
		};
		let hi = CandidateScore {
			year_match: YearMatch::Exact,
			region_match: RegionMatch::Unknown,
			variant_match: VariantMatch::Mismatch,
		};
		let sel = pick_best([(&a, lo), (&b, hi)]);
		match sel {
			Selection::Best(c) => assert_eq!(c.0, 2),
			_ => panic!(),
		}
	}

	#[test]
	fn pick_best_ambiguous_on_tie() {
		struct C;
		let a = C;
		let b = C;
		let s = CandidateScore {
			year_match: YearMatch::Exact,
			region_match: RegionMatch::Unknown,
			variant_match: VariantMatch::BothNone,
		};
		assert!(matches!(
			pick_best([(&a, s), (&b, s)]),
			Selection::Ambiguous
		));
	}

	#[test]
	fn pick_best_empty() {
		let empty: Vec<(&u32, CandidateScore)> = vec![];
		assert!(matches!(pick_best(empty), Selection::None));
	}

	#[test]
	fn pick_best_breaks_tie_via_region() {
		struct C(u32);
		let a = C(1);
		let b = C(2);
		let lo = CandidateScore {
			year_match: YearMatch::Exact,
			region_match: RegionMatch::Default,
			variant_match: VariantMatch::BothNone,
		};
		let hi = CandidateScore {
			year_match: YearMatch::Exact,
			region_match: RegionMatch::Exact,
			variant_match: VariantMatch::BothNone,
		};
		match pick_best([(&a, lo), (&b, hi)]) {
			Selection::Best(c) => assert_eq!(c.0, 2),
			_ => panic!(),
		}
	}
}
