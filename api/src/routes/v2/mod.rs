use crate::error::Error;
use crate::model::pagination::{
	CursorDecode, CursorError, CursorPosition, Page, PageMeta, PageParams, decode_cursor,
	encode_cursor,
};
use actix_web::HttpResponse;
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

pub mod bulk_by_id;
pub mod company;
pub mod dat_file;
pub mod error;
pub mod game;
pub mod identify;
pub mod identify_bulk;
pub mod platform;
pub mod signature_group;
pub mod stats;
pub mod suggestion;
pub mod user;

/// Unwrap a keyset resolve into its start position, or short-circuit the handler
/// by returning the v2 error envelope the resolve produced. Handlers stay
/// `error::Result<impl Responder>`; only the cursor-error path returns early with
/// a 400 envelope `HttpResponse` instead of threading it through `?`.
macro_rules! resolve_or_return {
	($resolve:expr) => {
		match $resolve {
			Ok(start) => start,
			Err(resp) => return Ok(resp),
		}
	};
}

pub(crate) use resolve_or_return;

/// Trim a search query and reject an empty one with the shared v2 `empty_query`
/// 400. Returns the trimmed slice on success so every search handler applies the
/// same guard and error envelope. Pair with [`resolve_or_return`] to short-circuit
/// the handler on the empty case.
pub fn require_non_empty_query(query: &str) -> Result<&str, HttpResponse> {
	let trimmed = query.trim();
	if trimmed.is_empty() {
		return Err(error::v2_bad_request(
			"empty_query",
			"search query must not be empty",
		));
	}
	Ok(trimmed)
}

/// Maps a typed keyset sort key to and from the opaque `sort_key` bytes carried
/// in a cursor. Each implementation owns one wire layout and the layouts must
/// never change: utf8 for name keys, f64 big-endian for score keys, i64 micros
/// for time keys. A position that cannot be reconstructed is a malformed cursor.
pub trait SortKey: Sized {
	fn to_sort_bytes(&self) -> Vec<u8>;
	fn from_sort_bytes(bytes: Vec<u8>) -> Result<Self, Error>;
}

impl SortKey for String {
	fn to_sort_bytes(&self) -> Vec<u8> {
		self.clone().into_bytes()
	}

	fn from_sort_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
		String::from_utf8(bytes)
			.map_err(|_| Error::BadRequest("malformed pagination cursor".into()))
	}
}

impl SortKey for f64 {
	fn to_sort_bytes(&self) -> Vec<u8> {
		self.to_be_bytes().to_vec()
	}

	fn from_sort_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
		let bytes: [u8; 8] = bytes
			.as_slice()
			.try_into()
			.map_err(|_| Error::BadRequest("malformed pagination cursor".into()))?;
		Ok(f64::from_be_bytes(bytes))
	}
}

impl SortKey for DateTime<Utc> {
	fn to_sort_bytes(&self) -> Vec<u8> {
		self.timestamp_micros().to_be_bytes().to_vec()
	}

	fn from_sort_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
		let bytes: [u8; 8] = bytes
			.as_slice()
			.try_into()
			.map_err(|_| Error::BadRequest("malformed pagination cursor".into()))?;
		let micros = i64::from_be_bytes(bytes);
		DateTime::<Utc>::from_timestamp_micros(micros)
			.ok_or_else(|| Error::BadRequest("malformed pagination cursor".into()))
	}
}

/// A keyset position resolved from a client request: either seek past `after`,
/// or start at page 1 (cursor absent, or a stale wire format that asks for a
/// fresh start under the current filters). Generic over the sort-key type `K`,
/// which determines the cursor's `sort_key` byte layout via [`SortKey`].
#[derive(Debug)]
pub enum KeysetStart<K = String> {
	First,
	After((K, Uuid)),
}

impl<K> KeysetStart<K> {
	pub fn after(self) -> Option<(K, Uuid)> {
		match self {
			KeysetStart::First => None,
			KeysetStart::After(pos) => Some(pos),
		}
	}

	pub fn has_previous(&self) -> bool {
		matches!(self, KeysetStart::After(_))
	}
}

/// A keyset position over a fuzzy-search score: seek past `(score, id)`, or start
/// at page 1.
pub type ScoreKeysetStart = KeysetStart<f64>;

/// A keyset position over a creation timestamp: seek past `(created_at, id)`, or
/// start at page 1.
pub type TimeKeysetStart = KeysetStart<DateTime<Utc>>;

/// Anything a v2 keyset list can page over. Yields the `(name, id)` pair that
/// becomes the next cursor's position.
pub trait KeysetRow {
	fn keyset_position(&self) -> (String, Uuid);
}

/// Resolve the request's cursor into a starting position for the `filter_tag`'d
/// sort+filter set. A malformed cursor is a 400; a tag mismatch is a 400 with a
/// restart hint; an unrecognized wire format silently restarts at page 1.
///
/// On a client cursor error the `Err` carries the ready-to-send v2 error
/// envelope rather than the shared `Error`, so the malformed and restart bodies
/// reach the client as `application/json` without passing through v1's
/// plain-text `ResponseError`.
///
/// Generic over the sort-key type. The name-, score-, and time-keyed resolvers
/// are thin specializations that pin `K`.
pub fn resolve_keyset_start_for<K: SortKey>(
	params: &PageParams,
	filter_tag: u64,
) -> Result<KeysetStart<K>, HttpResponse> {
	let Some(token) = params.cursor.as_deref().filter(|c| !c.is_empty()) else {
		return Ok(KeysetStart::First);
	};

	match decode_cursor(token, filter_tag) {
		Ok(CursorDecode::Position(pos)) => {
			let key = K::from_sort_bytes(pos.sort_key).map_err(|_| malformed_cursor_error())?;
			Ok(KeysetStart::After((key, pos.id)))
		}
		Ok(CursorDecode::Restart) => Ok(KeysetStart::First),
		Err(CursorError::Malformed) => Err(malformed_cursor_error()),
		Err(CursorError::FilterMismatch) => Err(cursor_mismatch_error()),
	}
}

/// Resolve a `(name, id)` keyset cursor. See [`resolve_keyset_start_for`].
pub fn resolve_keyset_start(
	params: &PageParams,
	filter_tag: u64,
) -> Result<KeysetStart, HttpResponse> {
	resolve_keyset_start_for::<String>(params, filter_tag)
}

/// Resolve a `(score, id)` keyset cursor. Same error contract as
/// [`resolve_keyset_start`].
pub fn resolve_score_keyset_start(
	params: &PageParams,
	filter_tag: u64,
) -> Result<ScoreKeysetStart, HttpResponse> {
	resolve_keyset_start_for::<f64>(params, filter_tag)
}

/// Resolve a `(created_at, id)` keyset cursor. Same error contract as
/// [`resolve_keyset_start`].
pub fn resolve_time_keyset_start(
	params: &PageParams,
	filter_tag: u64,
) -> Result<TimeKeysetStart, HttpResponse> {
	resolve_keyset_start_for::<DateTime<Utc>>(params, filter_tag)
}

fn malformed_cursor_error() -> HttpResponse {
	error::v2_bad_request("malformed_cursor", "malformed pagination cursor")
}

fn cursor_mismatch_error() -> HttpResponse {
	error::v2_cursor_restart(
		"cursor_filter_mismatch",
		"pagination cursor does not match the current filters; restart from page 1",
	)
}

/// Cursor encoding the final `(sort_key, id)` position when more rows follow,
/// otherwise `None`. The single point where a typed sort key is serialized into
/// the cursor wire format, shared by every keyset page builder.
fn next_cursor_from_position<K: SortKey>(
	position: Option<&(K, Uuid)>,
	has_more: bool,
	filter_tag: u64,
) -> Option<String> {
	if !has_more {
		return None;
	}
	position
		.map(|(key, id)| encode_cursor(filter_tag, &CursorPosition::new(key.to_sort_bytes(), *id)))
}

/// Cursor for the next page derived from the last row's [`KeysetRow`] position.
/// Pure encode path, exercised directly by the round-trip unit tests.
fn next_cursor_for<T: KeysetRow>(rows: &[T], has_more: bool, filter_tag: u64) -> Option<String> {
	let last = rows.last().map(KeysetRow::keyset_position);
	next_cursor_from_position(last.as_ref(), has_more, filter_tag)
}

/// Assemble the response envelope for a keyset page from an already-minted
/// `next_cursor`. `total` is honored only by callers that opted in on a bounded
/// reference table; pass `None` elsewhere.
fn build_keyset_page<T: Serialize>(
	rows: Vec<T>,
	next_cursor: Option<String>,
	has_more: bool,
	has_previous: bool,
	limit: u64,
	total: Option<u64>,
) -> HttpResponse {
	let page = Page::new(
		rows,
		PageMeta {
			limit,
			has_next_page: has_more,
			has_previous_page: has_previous,
			next_cursor,
			total_items: total,
		},
	);

	HttpResponse::Ok().json(page)
}

/// Assemble a `(name, id)` keyset page. The next cursor is derived from the last
/// row's [`KeysetRow::keyset_position`]. `total` is honored only by callers that
/// opted in on a bounded reference table; pass `None` everywhere else.
pub fn build_page<T: KeysetRow + Serialize>(
	rows: Vec<T>,
	has_more: bool,
	has_previous: bool,
	limit: u64,
	filter_tag: u64,
	total: Option<u64>,
) -> HttpResponse {
	let next_cursor = next_cursor_for(&rows, has_more, filter_tag);
	build_keyset_page(rows, next_cursor, has_more, has_previous, limit, total)
}

/// Assemble a `(score, id)` keyset page. `next_cursor` encodes the last row's
/// score and id when more rows follow.
pub fn build_score_page<T: Serialize>(
	rows: Vec<T>,
	positions: &[(f64, Uuid)],
	has_more: bool,
	has_previous: bool,
	limit: u64,
	filter_tag: u64,
) -> HttpResponse {
	let next_cursor = next_cursor_from_position(positions.last(), has_more, filter_tag);
	build_keyset_page(rows, next_cursor, has_more, has_previous, limit, None)
}

/// Assemble a `(created_at, id)` keyset page. `next_cursor` encodes the last
/// row's timestamp (microsecond precision) and id when more rows follow.
pub fn build_time_page<T: Serialize>(
	rows: Vec<T>,
	positions: &[(DateTime<Utc>, Uuid)],
	has_more: bool,
	has_previous: bool,
	limit: u64,
	filter_tag: u64,
) -> HttpResponse {
	let next_cursor = next_cursor_from_position(positions.last(), has_more, filter_tag);
	build_keyset_page(rows, next_cursor, has_more, has_previous, limit, None)
}

impl KeysetRow for service::model::CompanyMetadataResponse {
	fn keyset_position(&self) -> (String, Uuid) {
		(self.name.clone(), self.id)
	}
}

impl KeysetRow for service::model::PlatformMetadataResponse {
	fn keyset_position(&self) -> (String, Uuid) {
		(self.name.clone(), self.id)
	}
}

impl KeysetRow for service::model::PlaymatchSignatureGroup {
	fn keyset_position(&self) -> (String, Uuid) {
		(self.name.clone(), self.id)
	}
}

impl KeysetRow for service::model::PlaymatchSignatureGroupV2 {
	fn keyset_position(&self) -> (String, Uuid) {
		(self.name.clone(), self.id)
	}
}

impl KeysetRow for service::model::GameMetadataResponse {
	fn keyset_position(&self) -> (String, Uuid) {
		(self.name.clone(), self.id)
	}
}

impl KeysetRow for service::entities::dat_file::DatFileSummary {
	fn keyset_position(&self) -> (String, Uuid) {
		(self.name.clone(), self.id)
	}
}

impl KeysetRow for service::entities::dat_file::DatFileGame {
	fn keyset_position(&self) -> (String, Uuid) {
		(self.name.clone(), self.id)
	}
}

impl KeysetRow for service::model::PlaymatchGameFile {
	fn keyset_position(&self) -> (String, Uuid) {
		(self.file_name.clone(), self.id)
	}
}

impl KeysetRow for service::model::PlaymatchGameFileV2 {
	fn keyset_position(&self) -> (String, Uuid) {
		(self.file_name.clone(), self.id)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::model::pagination::{CursorDecode, decode_cursor};

	const TAG: u64 = 0x1122334455667788;

	struct Row {
		name: String,
		id: Uuid,
	}

	impl KeysetRow for Row {
		fn keyset_position(&self) -> (String, Uuid) {
			(self.name.clone(), self.id)
		}
	}

	fn params(cursor: Option<&str>) -> PageParams {
		PageParams {
			limit: None,
			cursor: cursor.map(str::to_string),
			with_total: None,
		}
	}

	#[test]
	fn absent_or_empty_cursor_starts_at_page_one() {
		assert!(matches!(
			resolve_keyset_start(&params(None), TAG).unwrap(),
			KeysetStart::First
		));
		assert!(matches!(
			resolve_keyset_start(&params(Some("")), TAG).unwrap(),
			KeysetStart::First
		));
	}

	#[test]
	fn next_cursor_round_trips_into_a_seek_position() {
		let id = Uuid::new_v4();
		let rows = vec![
			Row {
				name: "alpha".into(),
				id: Uuid::new_v4(),
			},
			Row {
				name: "omega".into(),
				id,
			},
		];

		let token = next_cursor_for(&rows, true, TAG).expect("has_more yields a cursor");

		match resolve_keyset_start(&params(Some(&token)), TAG).unwrap() {
			KeysetStart::After((name, decoded_id)) => {
				assert_eq!(name, "omega");
				assert_eq!(decoded_id, id);
			}
			KeysetStart::First => panic!("a valid cursor must seek, not restart"),
		}
	}

	#[test]
	fn no_cursor_emitted_on_the_final_page() {
		let rows = vec![Row {
			name: "only".into(),
			id: Uuid::new_v4(),
		}];
		assert!(next_cursor_for(&rows, false, TAG).is_none());
	}

	async fn error_body(resp: HttpResponse) -> serde_json::Value {
		let body = actix_web::body::to_bytes(resp.into_body()).await.unwrap();
		serde_json::from_slice(&body).unwrap()
	}

	#[actix_web::test]
	async fn cursor_minted_for_another_filter_set_is_rejected() {
		let rows = vec![Row {
			name: "x".into(),
			id: Uuid::new_v4(),
		}];
		let token = next_cursor_for(&rows, true, TAG).unwrap();

		let resp = resolve_keyset_start(&params(Some(&token)), TAG ^ 0xFF).unwrap_err();
		assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
		let json = error_body(resp).await;
		assert_eq!(json["code"], "cursor_filter_mismatch");
		assert_eq!(json["restart"], true);
	}

	#[actix_web::test]
	async fn malformed_cursor_is_a_plain_bad_request() {
		let resp = resolve_keyset_start(&params(Some("!!!not-base64!!!")), TAG).unwrap_err();
		assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
		let json = error_body(resp).await;
		assert_eq!(json["code"], "malformed_cursor");
	}

	#[test]
	fn unknown_wire_format_silently_restarts() {
		// A cursor whose format byte the codec no longer recognizes must not 400;
		// it must fall back to page 1 so a deploy does not break in-flight paging.
		let mut raw = Vec::new();
		raw.push(0xEE);
		raw.extend_from_slice(&TAG.to_be_bytes());
		raw.extend_from_slice(&3u16.to_be_bytes());
		raw.extend_from_slice(b"key");
		raw.extend_from_slice(Uuid::new_v4().as_bytes());
		let token = base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, raw);

		assert!(matches!(
			decode_cursor(&token, TAG),
			Ok(CursorDecode::Restart)
		));
		assert!(matches!(
			resolve_keyset_start(&params(Some(&token)), TAG).unwrap(),
			KeysetStart::First
		));
	}

	fn encode_score_cursor(score: f64, id: Uuid, tag: u64) -> String {
		encode_cursor(tag, &CursorPosition::new(score.to_be_bytes().to_vec(), id))
	}

	fn encode_time_cursor(ts: DateTime<Utc>, id: Uuid, tag: u64) -> String {
		encode_cursor(
			tag,
			&CursorPosition::new(ts.timestamp_micros().to_be_bytes().to_vec(), id),
		)
	}

	#[test]
	fn score_cursor_round_trips_into_a_seek_position() {
		let id = Uuid::new_v4();
		let token = encode_score_cursor(0.4375, id, TAG);

		match resolve_score_keyset_start(&params(Some(&token)), TAG).unwrap() {
			ScoreKeysetStart::After((score, decoded_id)) => {
				assert_eq!(score, 0.4375);
				assert_eq!(decoded_id, id);
			}
			ScoreKeysetStart::First => panic!("a valid score cursor must seek, not restart"),
		}
	}

	#[actix_web::test]
	async fn score_cursor_for_another_filter_set_is_rejected() {
		let token = encode_score_cursor(0.9, Uuid::new_v4(), TAG);
		let resp = resolve_score_keyset_start(&params(Some(&token)), TAG ^ 0xFF).unwrap_err();
		assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
		let json = error_body(resp).await;
		assert_eq!(json["code"], "cursor_filter_mismatch");
		assert_eq!(json["restart"], true);
	}

	#[test]
	fn score_page_emits_cursor_only_when_more_follow() {
		let positions = vec![(0.8_f64, Uuid::new_v4()), (0.6_f64, Uuid::new_v4())];
		let resp = build_score_page(vec![1, 2], &positions, true, false, 2, TAG);
		assert_eq!(resp.status(), actix_web::http::StatusCode::OK);

		let last = positions.last().copied().unwrap();
		let token = encode_score_cursor(last.0, last.1, TAG);
		match resolve_score_keyset_start(&params(Some(&token)), TAG).unwrap() {
			ScoreKeysetStart::After((score, id)) => {
				assert_eq!(score, last.0);
				assert_eq!(id, last.1);
			}
			ScoreKeysetStart::First => panic!("expected a seek position"),
		}
	}

	#[test]
	fn time_cursor_round_trips_with_microsecond_precision() {
		let id = Uuid::new_v4();
		let ts = DateTime::<Utc>::from_timestamp_micros(1_700_000_000_123_456).unwrap();
		let token = encode_time_cursor(ts, id, TAG);

		match resolve_time_keyset_start(&params(Some(&token)), TAG).unwrap() {
			TimeKeysetStart::After((decoded_ts, decoded_id)) => {
				assert_eq!(decoded_ts, ts);
				assert_eq!(decoded_id, id);
			}
			TimeKeysetStart::First => panic!("a valid time cursor must seek, not restart"),
		}
	}

	#[actix_web::test]
	async fn time_cursor_for_another_filter_set_is_rejected() {
		let ts = DateTime::<Utc>::from_timestamp_micros(1_700_000_000_000_000).unwrap();
		let token = encode_time_cursor(ts, Uuid::new_v4(), TAG);
		let resp = resolve_time_keyset_start(&params(Some(&token)), TAG ^ 0xAB).unwrap_err();
		assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
		let json = error_body(resp).await;
		assert_eq!(json["code"], "cursor_filter_mismatch");
		assert_eq!(json["restart"], true);
	}

	#[actix_web::test]
	async fn malformed_score_and_time_cursors_are_bad_requests() {
		for resp in [
			resolve_score_keyset_start(&params(Some("###")), TAG).unwrap_err(),
			resolve_time_keyset_start(&params(Some("###")), TAG).unwrap_err(),
		] {
			assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
			let json = error_body(resp).await;
			assert_eq!(json["code"], "malformed_cursor");
		}
	}

	#[test]
	fn absent_cursor_starts_score_and_time_at_page_one() {
		assert!(matches!(
			resolve_score_keyset_start(&params(None), TAG).unwrap(),
			ScoreKeysetStart::First
		));
		assert!(matches!(
			resolve_time_keyset_start(&params(None), TAG).unwrap(),
			TimeKeysetStart::First
		));
	}
}
