use sea_orm::{ConnectionTrait, Cursor, DbErr, SelectorTrait};

/// Hard ceiling on a single keyset page. Requests above this are clamped.
pub const MAX_PAGE_LIMIT: u64 = 50;
pub const DEFAULT_PAGE_LIMIT: u64 = 25;

/// Clamp a requested page limit into `[1, MAX_PAGE_LIMIT]`, substituting
/// [`DEFAULT_PAGE_LIMIT`] for an absent or zero value. Out-of-range values are
/// clamped rather than rejected so a list endpoint never 400s on `limit` alone.
pub fn clamp_page_limit(requested: Option<u64>) -> u64 {
	match requested {
		None | Some(0) => DEFAULT_PAGE_LIMIT,
		Some(n) => n.min(MAX_PAGE_LIMIT),
	}
}

#[derive(Debug, Clone)]
pub struct KeysetPage<T> {
	pub rows: Vec<T>,
	pub has_more: bool,
}

impl<T> KeysetPage<T> {
	pub fn map_rows<U, F: FnMut(T) -> U>(self, f: F) -> KeysetPage<U> {
		KeysetPage {
			rows: self.rows.into_iter().map(f).collect(),
			has_more: self.has_more,
		}
	}
}

/// Fetch one keyset page from an already-positioned [`Cursor`].
///
/// The caller is responsible for the `cursor_by(...)` ordering tuple and any
/// `after(...)` seeking. This applies the clamp and the N+1 trick: it requests
/// `limit + 1` rows, infers `has_more` from the overflow row, then truncates so
/// the page never exceeds `limit`. No `COUNT(*)` is issued.
pub async fn fetch_keyset_page<S, C>(
	cursor: &mut Cursor<S>,
	requested_limit: Option<u64>,
	conn: &C,
) -> Result<KeysetPage<S::Item>, DbErr>
where
	S: SelectorTrait,
	C: ConnectionTrait,
{
	let limit = clamp_page_limit(requested_limit);
	let mut rows = cursor.first(limit + 1).all(conn).await?;
	let has_more = rows.len() as u64 > limit;
	if has_more {
		rows.truncate(limit as usize);
	}
	Ok(KeysetPage { rows, has_more })
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn clamp_substitutes_default_for_absent_or_zero() {
		assert_eq!(clamp_page_limit(None), DEFAULT_PAGE_LIMIT);
		assert_eq!(clamp_page_limit(Some(0)), DEFAULT_PAGE_LIMIT);
	}

	#[test]
	fn clamp_caps_at_max_and_passes_in_range() {
		assert_eq!(clamp_page_limit(Some(1)), 1);
		assert_eq!(clamp_page_limit(Some(25)), 25);
		assert_eq!(clamp_page_limit(Some(50)), 50);
		assert_eq!(clamp_page_limit(Some(51)), MAX_PAGE_LIMIT);
		assert_eq!(clamp_page_limit(Some(u64::MAX)), MAX_PAGE_LIMIT);
	}
}
