use crate::matching::name_parse::ParsedName;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateVerdict {
	AcceptPreferred,
	Accept,
	Reject,
}

/// Rejects candidates whose year is >=2 off from the DAT year, or whose
/// platform list does not include the DAT's platform id. Returns
/// `AcceptPreferred` on an exact year match. When either side lacks the
/// data, the corresponding check is skipped.
pub fn score_candidate(
	parsed_dat: &ParsedName,
	candidate_year: Option<u16>,
	candidate_platforms: Option<&[i64]>,
	dat_platform_id: Option<i64>,
) -> CandidateVerdict {
	if let Some(platforms) = candidate_platforms
		&& !platforms.is_empty()
		&& let Some(dat_pid) = dat_platform_id
		&& !platforms.contains(&dat_pid)
	{
		return CandidateVerdict::Reject;
	}

	match (parsed_dat.year, candidate_year) {
		(Some(dat_year), Some(cand_year)) => {
			let delta = dat_year.abs_diff(cand_year);
			if delta >= 2 {
				CandidateVerdict::Reject
			} else if delta == 0 {
				CandidateVerdict::AcceptPreferred
			} else {
				CandidateVerdict::Accept
			}
		}
		_ => CandidateVerdict::Accept,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn dat(year: Option<u16>) -> ParsedName {
		ParsedName {
			year,
			..ParsedName::default()
		}
	}

	#[test]
	fn no_year_no_platform_accepts() {
		assert_eq!(
			score_candidate(&dat(None), None, None, None),
			CandidateVerdict::Accept
		);
	}

	#[test]
	fn exact_year_preferred() {
		assert_eq!(
			score_candidate(&dat(Some(2000)), Some(2000), None, None),
			CandidateVerdict::AcceptPreferred
		);
	}

	#[test]
	fn off_by_one_accepts() {
		assert_eq!(
			score_candidate(&dat(Some(2000)), Some(2001), None, None),
			CandidateVerdict::Accept
		);
		assert_eq!(
			score_candidate(&dat(Some(2000)), Some(1999), None, None),
			CandidateVerdict::Accept
		);
	}

	#[test]
	fn off_by_two_rejects() {
		assert_eq!(
			score_candidate(&dat(Some(2000)), Some(2002), None, None),
			CandidateVerdict::Reject
		);
		assert_eq!(
			score_candidate(&dat(Some(2000)), Some(1998), None, None),
			CandidateVerdict::Reject
		);
	}

	#[test]
	fn off_by_three_rejects() {
		assert_eq!(
			score_candidate(&dat(Some(2000)), Some(2003), None, None),
			CandidateVerdict::Reject
		);
	}

	#[test]
	fn missing_dat_year_skips_check() {
		assert_eq!(
			score_candidate(&dat(None), Some(2010), None, None),
			CandidateVerdict::Accept
		);
	}

	#[test]
	fn missing_candidate_year_skips_check() {
		assert_eq!(
			score_candidate(&dat(Some(2000)), None, None, None),
			CandidateVerdict::Accept
		);
	}

	#[test]
	fn empty_candidate_platforms_skipped() {
		assert_eq!(
			score_candidate(&dat(None), None, Some(&[]), Some(7)),
			CandidateVerdict::Accept
		);
	}

	#[test]
	fn platform_match_passes() {
		assert_eq!(
			score_candidate(&dat(None), None, Some(&[3, 7, 11]), Some(7)),
			CandidateVerdict::Accept
		);
	}

	#[test]
	fn platform_mismatch_rejects() {
		assert_eq!(
			score_candidate(&dat(None), None, Some(&[3, 11]), Some(7)),
			CandidateVerdict::Reject
		);
	}

	#[test]
	fn missing_dat_platform_skips_check() {
		assert_eq!(
			score_candidate(&dat(None), None, Some(&[3, 11]), None),
			CandidateVerdict::Accept
		);
	}

	#[test]
	fn platform_reject_short_circuits_year() {
		assert_eq!(
			score_candidate(&dat(Some(2000)), Some(2000), Some(&[1]), Some(7)),
			CandidateVerdict::Reject
		);
	}

	#[test]
	fn platform_pass_year_exact() {
		assert_eq!(
			score_candidate(&dat(Some(2000)), Some(2000), Some(&[7, 8]), Some(7)),
			CandidateVerdict::AcceptPreferred
		);
	}
}
