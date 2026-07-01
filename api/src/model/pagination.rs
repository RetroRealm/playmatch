use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};
use service::entities::dat_file::{DatFileGame, DatFileImportTimelineEntry, DatFileSummary};
use service::model::suggestion::Suggestion;
use service::model::{
	CompanyMetadataResponse, GameMetadataResponse, GameNameSearchResultV2,
	PlatformMetadataResponse, PlaymatchGameFileV2, PlaymatchSignatureGroupV2,
};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// Hard ceiling on a single page. Requests above this are clamped, never rejected.
pub const MAX_PAGE_LIMIT: u64 = 50;
/// Page size used when the client omits `limit` or sends `0`.
pub const DEFAULT_PAGE_LIMIT: u64 = 25;

/// Wire format version of the cursor payload. Bump this when the byte layout
/// changes. A cursor carrying an unrecognized version is not an error: callers
/// treat it as "start over from page 1" so in-flight pagination survives a deploy.
const CURSOR_FORMAT_VERSION: u8 = 1;

/// FNV-1a offset basis, the seed for a fresh filter-tag hash.
pub const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// FNV-1a over arbitrary bytes. A fixed algorithm (not `DefaultHasher`, whose
/// output is not stable across toolchains) keeps a cursor's filter tag identical
/// across deploys, so paging only restarts when the filters actually change.
pub fn fnv1a(seed: u64, bytes: &[u8]) -> u64 {
	let mut hash = seed;
	for b in bytes {
		hash ^= u64::from(*b);
		hash = hash.wrapping_mul(FNV_PRIME);
	}
	hash
}

pub fn fnv1a_uuid(seed: u64, value: Option<Uuid>) -> u64 {
	match value {
		Some(id) => fnv1a(seed ^ 1, id.as_bytes()),
		None => fnv1a(seed, &[0]),
	}
}

/// One page of results together with its pagination metadata.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[aliases(
	PageOfCompanyMetadataResponse = Page<CompanyMetadataResponse>,
	PageOfPlatformMetadataResponse = Page<PlatformMetadataResponse>,
	PageOfPlaymatchSignatureGroup = Page<PlaymatchSignatureGroupV2>,
	PageOfGameMetadataResponse = Page<GameMetadataResponse>,
	PageOfGameNameSearchResult = Page<GameNameSearchResultV2>,
	PageOfSuggestion = Page<Suggestion>,
	PageOfDatFileSummary = Page<DatFileSummary>,
	PageOfDatFileGame = Page<DatFileGame>,
	PageOfDatFileImportTimelineEntry = Page<DatFileImportTimelineEntry>,
	PageOfPlaymatchGameFile = Page<PlaymatchGameFileV2>,
)]
pub struct Page<T> {
	/// The items on this page, in the endpoint's sort order.
	pub data: Vec<T>,

	/// Page size, next-page cursor, and optional total for this page.
	pub pagination: PageMeta,
}

impl<T> Page<T> {
	pub fn new(data: Vec<T>, meta: PageMeta) -> Self {
		Self {
			data,
			pagination: meta,
		}
	}
}

/// Pagination metadata returned alongside every list page.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PageMeta {
	/// The page size applied to this page after clamping.
	pub limit: u64,

	/// Whether another page of results follows this one.
	pub has_next_page: bool,

	/// Whether this page was reached from an earlier one, that is, whether a cursor was supplied.
	pub has_previous_page: bool,

	/// The cursor to pass as `cursor` to fetch the next page. Absent on the last page.
	pub next_cursor: Option<String>,

	/// The exact total number of items across all pages, serialized as `totalItems`. Present only when `withTotal` was requested and the endpoint supports it (small bounded reference tables); omitted otherwise.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub total_items: Option<u64>,
}

/// Keyset-pagination query parameters shared by every v2 list endpoint.
#[derive(Debug, Clone, Default, Serialize, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct PageParams {
	/// Page size. Ranges from 1 to 50. Defaults to 25 when omitted or `0`. Values above 50 are clamped, never rejected.
	#[param(required = false, minimum = 1, maximum = 50)]
	pub limit: Option<u64>,

	/// Opaque cursor copied from the previous page's `nextCursor`. Omit it to fetch the first page. Keep the filters identical across pages.
	#[param(required = false)]
	pub cursor: Option<String>,

	/// Whether to include an exact `totalItems` count in the page metadata. Honored only on small bounded reference tables. Defaults to `false`.
	#[param(required = false)]
	pub with_total: Option<bool>,
}

impl PageParams {
	pub fn limit_clamped(&self) -> u64 {
		clamp_limit(self.limit)
	}

	pub fn wants_total(&self) -> bool {
		self.with_total.unwrap_or(false)
	}
}

/// Clamp a requested limit into `[1, MAX_PAGE_LIMIT]`, substituting the default
/// for an absent or zero value. Out-of-range inputs are clamped, never rejected.
pub fn clamp_limit(requested: Option<u64>) -> u64 {
	match requested {
		None | Some(0) => DEFAULT_PAGE_LIMIT,
		Some(n) => n.min(MAX_PAGE_LIMIT),
	}
}

/// The keyset position a cursor points at: the last row's `(sort_key, id)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorPosition {
	pub sort_key: Vec<u8>,
	pub id: Uuid,
}

impl CursorPosition {
	pub fn new(sort_key: impl Into<Vec<u8>>, id: Uuid) -> Self {
		Self {
			sort_key: sort_key.into(),
			id,
		}
	}
}

/// Outcome of decoding a client-supplied cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CursorDecode {
	/// A usable position for the current sort+filter set.
	Position(CursorPosition),
	/// A well-formed cursor from an older wire format. The client keeps its
	/// pagination intent but must restart at page 1 under the current filters.
	Restart,
}

/// Why a cursor could not be turned into a usable position. Both variants map to
/// HTTP 400; `FilterMismatch` additionally tells the client to restart at page 1.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CursorError {
	#[error("malformed pagination cursor")]
	Malformed,
	#[error("pagination cursor does not match the current filters; restart from page 1")]
	FilterMismatch,
}

/// Encode a keyset position into an opaque base64url token.
///
/// Layout (all integers big-endian):
///   [0]      format version byte
///   [1..9]   filter tag (u64) identifying the active sort + filter set
///   [9..11]  sort_key length (u16)
///   [11..]   sort_key bytes, then the 16-byte id
pub fn encode_cursor(filter_tag: u64, position: &CursorPosition) -> String {
	let sort_len = position.sort_key.len();
	let mut buf = Vec::with_capacity(11 + sort_len + 16);
	buf.push(CURSOR_FORMAT_VERSION);
	buf.extend_from_slice(&filter_tag.to_be_bytes());
	buf.extend_from_slice(&(sort_len as u16).to_be_bytes());
	buf.extend_from_slice(&position.sort_key);
	buf.extend_from_slice(position.id.as_bytes());
	URL_SAFE_NO_PAD.encode(buf)
}

/// Decode an opaque cursor produced by [`encode_cursor`].
///
/// `expected_tag` is the filter tag of the request currently being served.
/// A version mismatch yields [`CursorDecode::Restart`] (not an error) so a
/// deploy that bumps the format does not break clients mid-pagination. A
/// structurally broken token is [`CursorError::Malformed`]; a well-formed token
/// whose tag disagrees with `expected_tag` is [`CursorError::FilterMismatch`].
pub fn decode_cursor(token: &str, expected_tag: u64) -> Result<CursorDecode, CursorError> {
	let bytes = URL_SAFE_NO_PAD
		.decode(token.as_bytes())
		.map_err(|_| CursorError::Malformed)?;

	let version = *bytes.first().ok_or(CursorError::Malformed)?;
	if version != CURSOR_FORMAT_VERSION {
		return Ok(CursorDecode::Restart);
	}

	if bytes.len() < 11 {
		return Err(CursorError::Malformed);
	}

	let tag = u64::from_be_bytes(bytes[1..9].try_into().map_err(|_| CursorError::Malformed)?);
	let sort_len = u16::from_be_bytes(
		bytes[9..11]
			.try_into()
			.map_err(|_| CursorError::Malformed)?,
	) as usize;

	let sort_start = 11;
	let id_start = sort_start + sort_len;
	if bytes.len() != id_start + 16 {
		return Err(CursorError::Malformed);
	}

	if tag != expected_tag {
		return Err(CursorError::FilterMismatch);
	}

	let sort_key = bytes[sort_start..id_start].to_vec();
	let id = Uuid::from_slice(&bytes[id_start..]).map_err(|_| CursorError::Malformed)?;

	Ok(CursorDecode::Position(CursorPosition { sort_key, id }))
}

#[cfg(test)]
mod tests {
	use super::*;

	fn pos(sort: &[u8], id: Uuid) -> CursorPosition {
		CursorPosition::new(sort.to_vec(), id)
	}

	#[test]
	fn clamp_limit_substitutes_default_for_absent_or_zero() {
		assert_eq!(clamp_limit(None), DEFAULT_PAGE_LIMIT);
		assert_eq!(clamp_limit(Some(0)), DEFAULT_PAGE_LIMIT);
	}

	#[test]
	fn clamp_limit_caps_at_max_and_passes_through_in_range() {
		assert_eq!(clamp_limit(Some(1)), 1);
		assert_eq!(clamp_limit(Some(25)), 25);
		assert_eq!(clamp_limit(Some(50)), 50);
		assert_eq!(clamp_limit(Some(51)), MAX_PAGE_LIMIT);
		assert_eq!(clamp_limit(Some(u64::MAX)), MAX_PAGE_LIMIT);
	}

	#[test]
	fn page_params_helpers_mirror_clamp_and_total_flag() {
		let p = PageParams {
			limit: Some(1000),
			cursor: None,
			with_total: Some(true),
		};
		assert_eq!(p.limit_clamped(), MAX_PAGE_LIMIT);
		assert!(p.wants_total());
		assert!(!PageParams::default().wants_total());
	}

	#[test]
	fn cursor_round_trips() {
		let id = Uuid::new_v4();
		let position = pos(b"some-sort-key", id);
		let token = encode_cursor(7, &position);

		match decode_cursor(&token, 7).unwrap() {
			CursorDecode::Position(decoded) => assert_eq!(decoded, position),
			other => panic!("expected position, got {other:?}"),
		}
	}

	#[test]
	fn cursor_round_trips_with_empty_sort_key() {
		let id = Uuid::new_v4();
		let position = pos(b"", id);
		let token = encode_cursor(0, &position);

		assert_eq!(
			decode_cursor(&token, 0).unwrap(),
			CursorDecode::Position(position)
		);
	}

	#[test]
	fn cursor_is_url_safe_and_unpadded() {
		let token = encode_cursor(u64::MAX, &pos(&[0xff; 40], Uuid::nil()));
		assert!(!token.contains('='));
		assert!(!token.contains('+'));
		assert!(!token.contains('/'));
	}

	#[test]
	fn malformed_base64_is_rejected() {
		assert_eq!(
			decode_cursor("not base64!!!", 0),
			Err(CursorError::Malformed)
		);
	}

	#[test]
	fn truncated_payload_is_rejected() {
		let id = Uuid::new_v4();
		let token = encode_cursor(3, &pos(b"abc", id));
		let mut raw = URL_SAFE_NO_PAD.decode(token).unwrap();
		raw.truncate(raw.len() - 4);
		let truncated = URL_SAFE_NO_PAD.encode(raw);
		assert_eq!(decode_cursor(&truncated, 3), Err(CursorError::Malformed));
	}

	#[test]
	fn empty_token_is_malformed() {
		assert_eq!(decode_cursor("", 0), Err(CursorError::Malformed));
	}

	#[test]
	fn declared_sort_length_overrunning_buffer_is_rejected() {
		let mut raw = Vec::new();
		raw.push(CURSOR_FORMAT_VERSION);
		raw.extend_from_slice(&5u64.to_be_bytes());
		raw.extend_from_slice(&9000u16.to_be_bytes());
		raw.extend_from_slice(b"short");
		let token = URL_SAFE_NO_PAD.encode(raw);
		assert_eq!(decode_cursor(&token, 5), Err(CursorError::Malformed));
	}

	#[test]
	fn filter_tag_mismatch_is_reported() {
		let token = encode_cursor(11, &pos(b"key", Uuid::new_v4()));
		assert_eq!(decode_cursor(&token, 22), Err(CursorError::FilterMismatch));
	}

	#[test]
	fn unknown_format_version_signals_restart_not_error() {
		let id = Uuid::new_v4();
		let mut raw = Vec::new();
		raw.push(CURSOR_FORMAT_VERSION.wrapping_add(99));
		raw.extend_from_slice(&5u64.to_be_bytes());
		raw.extend_from_slice(&3u16.to_be_bytes());
		raw.extend_from_slice(b"key");
		raw.extend_from_slice(id.as_bytes());
		let token = URL_SAFE_NO_PAD.encode(raw);

		assert_eq!(decode_cursor(&token, 5), Ok(CursorDecode::Restart));
		assert_eq!(decode_cursor(&token, 999), Ok(CursorDecode::Restart));
	}
}
