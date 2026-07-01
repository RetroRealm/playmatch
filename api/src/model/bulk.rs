use serde::{Deserialize, Serialize};
pub use service::bulk::MAX_BULK_ITEMS;
use service::model::GameFileMatchSearch;
use utoipa::ToSchema;
use uuid::Uuid;

/// Longest accepted caller-supplied correlation key. Kept small because the key
/// is echoed back per item and only needs to disambiguate within a single
/// batch.
pub const MAX_BULK_KEY_LEN: usize = 128;

/// One entry in a bulk identify request: the file search fields plus an optional
/// caller `key` for correlating results without relying on array position.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BulkIdentifyItem {
	#[serde(flatten)]
	pub search: GameFileMatchSearch,

	/// Optional caller key echoed back on the matching result. Unique within the batch, at most 128 characters.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub key: Option<String>,
}

/// Bulk identify request body. Up to 100 items per request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BulkIdentifyRequest {
	pub items: Vec<BulkIdentifyItem>,
}

/// Per-item processing status. `ok` means the lookup ran and a result is present,
/// which may still be a no-match; `invalid` means the item failed validation and
/// no lookup ran; `error` means a per-item failure while the batch as a whole
/// still returns 200.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum BulkItemStatus {
	Ok,
	Invalid,
	Error,
}

/// Cache outcome for an item. `HIT` when the result came from cache, `MISS` when it was computed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum BulkCache {
	Hit,
	Miss,
}

impl BulkCache {
	pub fn as_str(self) -> &'static str {
		match self {
			BulkCache::Hit => "HIT",
			BulkCache::Miss => "MISS",
		}
	}
}

/// Structured per-item error detail returned when `status` is `invalid` or
/// `error`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BulkItemError {
	/// A stable machine-readable error code for this item.
	pub code: String,

	/// The request field the error applies to, when the error is tied to one field.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub field: Option<String>,

	/// A human-readable explanation of the error.
	pub message: String,
}

/// One result in a bulk identify response, correlated to its request item by
/// echoed `index` and `key`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BulkIdentifyResult<T: Serialize> {
	/// The zero-based position of this item in the request array.
	pub index: usize,

	/// The caller key from the request item, when one was supplied.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub key: Option<String>,

	/// The processing status of this item.
	pub status: BulkItemStatus,

	/// Cache outcome of the identify lookup. Present only when `status` is `ok`.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub cache: Option<BulkCache>,

	/// The match result. A no-match is conveyed by a `gameMatchType` of `NoMatch` on the result, not by omission. Present only when `status` is `ok`.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub r#match: Option<T>,

	/// Error detail. Present only when `status` is `invalid` or `error`.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub error: Option<BulkItemError>,
}

/// Per-batch totals for a bulk identify response.
#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct BulkIdentifySummary {
	/// The number of items in the batch.
	pub total: usize,

	/// The number of items whose lookup ran successfully.
	pub succeeded: usize,

	/// The number of items that were invalid or errored.
	pub failed: usize,

	/// The number of successful items that resolved to a game.
	pub matched: usize,

	/// The number of successful items that resolved to no match.
	pub unmatched: usize,
}

/// Top-level bulk identify response. Returned with HTTP 200 once the batch passes
/// the size and shape check; per-item outcomes live in `results`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BulkIdentifyResponse<T: Serialize> {
	/// Per-batch totals across all items.
	pub summary: BulkIdentifySummary,

	/// One result per request item, in request order.
	pub results: Vec<BulkIdentifyResult<T>>,
}

/// The 400 body returned when a batch exceeds the item cap.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BatchTooLargeBody {
	/// A stable machine-readable error code.
	pub code: &'static str,

	/// The maximum number of items allowed per request.
	pub limit: usize,

	/// The number of items the request carried.
	pub received: usize,
}

/// Validation outcome for the batch envelope itself, before any item runs.
#[derive(Debug, PartialEq, Eq)]
pub enum BatchValidation {
	Ok,
	Empty,
	TooLarge { received: usize },
}

/// Gate the batch on size and emptiness. Per the bulk design an empty or missing
/// array is a 400, and a batch above the cap is a distinct structured 400. Pure
/// so it can be unit-tested without an HTTP layer.
pub fn validate_batch_size(len: usize) -> BatchValidation {
	if len == 0 {
		return BatchValidation::Empty;
	}
	if len > MAX_BULK_ITEMS {
		return BatchValidation::TooLarge { received: len };
	}
	BatchValidation::Ok
}

/// Reject a batch whose caller-supplied keys collide. Keys are optional, but any
/// present key must be unique within the batch and within length so result
/// correlation stays unambiguous. Returns the offending index on the first
/// violation.
pub fn validate_keys(items: &[BulkIdentifyItem]) -> Result<(), (usize, BulkItemError)> {
	let mut seen = std::collections::HashSet::new();
	for (index, item) in items.iter().enumerate() {
		let Some(key) = item.key.as_deref() else {
			continue;
		};
		if key.chars().count() > MAX_BULK_KEY_LEN {
			return Err((
				index,
				BulkItemError {
					code: "key_too_long".to_string(),
					field: Some("key".to_string()),
					message: format!("key exceeds {MAX_BULK_KEY_LEN} characters"),
				},
			));
		}
		if !seen.insert(key) {
			return Err((
				index,
				BulkItemError {
					code: "duplicate_key".to_string(),
					field: Some("key".to_string()),
					message: "key is not unique within the batch".to_string(),
				},
			));
		}
	}
	Ok(())
}

/// Bulk get-by-id request body. Up to 100 ids per request, correlated by id, so
/// no caller key is needed.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BulkIdsRequest {
	/// The ids to look up. Up to 100 per request; duplicate ids are resolved once.
	pub ids: Vec<Uuid>,
}

/// Per-item lookup status for a bulk get-by-id response. A missing id is a
/// per-item `not_found`, never a batch-level 404.
//
// Retained for the frozen v1 OpenAPI surface. v2 responses use
// [`BulkByIdStatusV2`] so the `notFound` value is camelCase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BulkByIdStatus {
	Ok,
	NotFound,
}

/// Per-item lookup status for a v2 bulk get-by-id response. A missing id is a
/// per-item `notFound`, never a batch-level 404.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum BulkByIdStatusV2 {
	Ok,
	NotFound,
}

/// One result in a bulk get-by-id response, correlated to its request by `id`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BulkByIdResult<T: Serialize> {
	/// The id this result corresponds to.
	pub id: Uuid,

	/// Whether the id resolved to a resource.
	pub status: BulkByIdStatusV2,

	/// The resolved resource. Present only when `status` is `ok`.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub data: Option<T>,
}

/// Per-batch totals for a bulk get-by-id response.
#[derive(Debug, Clone, Default, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BulkByIdSummary {
	/// The number of distinct ids looked up.
	pub total: usize,

	/// The number of ids that resolved to a resource.
	pub found: usize,

	/// The number of ids with no matching resource.
	pub not_found: usize,
}

/// Top-level bulk get-by-id response. Returned with HTTP 200 once the batch passes
/// the size and shape check; per-item outcomes live in `results`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BulkByIdResponse<T: Serialize> {
	/// Per-batch totals across all ids.
	pub summary: BulkByIdSummary,

	/// One result per distinct id, in first-seen order.
	pub results: Vec<BulkByIdResult<T>>,
}

/// Concrete schema mirrors of the generic bulk responses, used only to document
/// the 200 body of each bulk endpoint. utoipa 4 names a nested generic's `$ref`
/// by the base type and never substitutes the concrete payload, so a
/// `BulkByIdResponse<T>` alias renders a dangling `BulkByIdResult` ref with a `T`
/// payload. These non-generic types pin the payload per endpoint so the
/// documented shape resolves and matches the real JSON. They are never
/// constructed; the handlers keep returning the generic runtime types.
pub mod doc {
	use super::{
		BulkByIdStatusV2, BulkByIdSummary, BulkCache, BulkIdentifySummary, BulkItemError,
		BulkItemStatus,
	};
	use serde::Serialize;
	use service::entities::dat_file::DatFileDetail;
	use service::model::{
		CompanyMetadataResponse, GameAndRelationMatchResultV2, GameMetadataMatchResult,
		GameMetadataResponse, PlatformMetadataResponse, PlaymatchGameFileV2,
		PlaymatchSignatureGroupV2,
	};
	use utoipa::ToSchema;
	use uuid::Uuid;

	macro_rules! bulk_by_id_doc {
		($result:ident, $response:ident, $payload:ty, $entity:literal) => {
			#[doc = concat!("Documents one ", $entity, " result in a bulk get-by-id response.")]
			#[derive(Serialize, ToSchema)]
			pub struct $result {
				/// The id this result corresponds to.
				pub id: Uuid,
				/// Whether the id resolved to a resource.
				pub status: BulkByIdStatusV2,
				/// The resolved resource. Present only when `status` is `ok`.
				#[serde(skip_serializing_if = "Option::is_none")]
				pub data: Option<$payload>,
			}

			#[doc = concat!("Documents the 200 body of the bulk ", $entity, " get-by-id endpoint.")]
			#[derive(Serialize, ToSchema)]
			pub struct $response {
				/// Per-batch totals across all ids.
				pub summary: BulkByIdSummary,
				/// One result per distinct id, in first-seen order.
				pub results: Vec<$result>,
			}
		};
	}

	macro_rules! bulk_identify_doc {
		($result:ident, $response:ident, $payload:ty, $entity:literal) => {
			#[doc = concat!("Documents one ", $entity, " result in a bulk identify response.")]
			#[derive(Serialize, ToSchema)]
			pub struct $result {
				/// The zero-based position of this item in the request array.
				pub index: usize,
				/// The caller key from the request item, when one was supplied.
				#[serde(skip_serializing_if = "Option::is_none")]
				pub key: Option<String>,
				/// The processing status of this item.
				pub status: BulkItemStatus,
				/// Cache outcome of the identify lookup. Present only when `status` is `ok`.
				#[serde(skip_serializing_if = "Option::is_none")]
				pub cache: Option<BulkCache>,
				/// The match result. Present only when `status` is `ok`.
				#[serde(skip_serializing_if = "Option::is_none")]
				pub r#match: Option<$payload>,
				/// Error detail. Present only when `status` is `invalid` or `error`.
				#[serde(skip_serializing_if = "Option::is_none")]
				pub error: Option<BulkItemError>,
			}

			#[doc = concat!("Documents the 200 body of the bulk ", $entity, " identify endpoint.")]
			#[derive(Serialize, ToSchema)]
			pub struct $response {
				/// Per-batch totals across all items.
				pub summary: BulkIdentifySummary,
				/// One result per request item, in request order.
				pub results: Vec<$result>,
			}
		};
	}

	bulk_by_id_doc!(
		BulkGamesByIdResult,
		BulkGamesByIdResponse,
		GameMetadataResponse,
		"game"
	);
	bulk_by_id_doc!(
		BulkPlatformsByIdResult,
		BulkPlatformsByIdResponse,
		PlatformMetadataResponse,
		"platform"
	);
	bulk_by_id_doc!(
		BulkCompaniesByIdResult,
		BulkCompaniesByIdResponse,
		CompanyMetadataResponse,
		"company"
	);
	bulk_by_id_doc!(
		BulkDatFilesByIdResult,
		BulkDatFilesByIdResponse,
		DatFileDetail,
		"dat file"
	);
	bulk_by_id_doc!(
		BulkSignatureGroupsByIdResult,
		BulkSignatureGroupsByIdResponse,
		PlaymatchSignatureGroupV2,
		"signature group"
	);
	bulk_by_id_doc!(
		BulkGameFilesByIdResult,
		BulkGameFilesByIdResponse,
		PlaymatchGameFileV2,
		"game file"
	);

	bulk_identify_doc!(
		BulkIdentifyIdsResult,
		BulkIdentifyIdsResponse,
		GameMetadataMatchResult,
		"metadata-ids"
	);
	bulk_identify_doc!(
		BulkIdentifyRelationsResult,
		BulkIdentifyRelationsResponse,
		GameAndRelationMatchResultV2,
		"relations"
	);
}

/// Dedupe ids server-side while preserving first-occurrence order. The summary
/// `total` counts distinct ids, so a caller that repeats an id sees it resolved
/// once. Pure so it can be unit-tested without an HTTP layer.
pub fn dedupe_ids(ids: Vec<Uuid>) -> Vec<Uuid> {
	let mut seen = std::collections::HashSet::with_capacity(ids.len());
	let mut out = Vec::with_capacity(ids.len());
	for id in ids {
		if seen.insert(id) {
			out.push(id);
		}
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;

	fn item(key: Option<&str>) -> BulkIdentifyItem {
		BulkIdentifyItem {
			search: GameFileMatchSearch {
				file_name: "game.rom".to_string(),
				file_size: 1024,
				md5: None,
				sha1: None,
				sha256: None,
				crc: None,
			},
			key: key.map(str::to_string),
		}
	}

	#[test]
	fn empty_batch_is_rejected() {
		assert_eq!(validate_batch_size(0), BatchValidation::Empty);
	}

	#[test]
	fn batch_at_the_cap_is_accepted() {
		assert_eq!(validate_batch_size(MAX_BULK_ITEMS), BatchValidation::Ok);
	}

	#[test]
	fn batch_above_the_cap_reports_received_count() {
		assert_eq!(
			validate_batch_size(MAX_BULK_ITEMS + 1),
			BatchValidation::TooLarge {
				received: MAX_BULK_ITEMS + 1
			}
		);
	}

	#[test]
	fn distinct_and_absent_keys_pass() {
		let items = vec![item(Some("a")), item(None), item(Some("b")), item(None)];
		assert!(validate_keys(&items).is_ok());
	}

	#[test]
	fn duplicate_key_is_rejected_at_the_second_occurrence() {
		let items = vec![item(Some("dup")), item(None), item(Some("dup"))];
		let (index, err) = validate_keys(&items).unwrap_err();
		assert_eq!(index, 2);
		assert_eq!(err.code, "duplicate_key");
	}

	#[test]
	fn overlong_key_is_rejected() {
		let long = "k".repeat(MAX_BULK_KEY_LEN + 1);
		let items = vec![item(Some(&long))];
		let (index, err) = validate_keys(&items).unwrap_err();
		assert_eq!(index, 0);
		assert_eq!(err.code, "key_too_long");
	}

	#[test]
	fn dedupe_ids_keeps_first_occurrence_order() {
		let a = Uuid::new_v4();
		let b = Uuid::new_v4();
		let c = Uuid::new_v4();
		let deduped = dedupe_ids(vec![b, a, b, c, a]);
		assert_eq!(deduped, vec![b, a, c]);
	}

	#[test]
	fn dedupe_ids_on_distinct_input_is_identity() {
		let ids = vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
		assert_eq!(dedupe_ids(ids.clone()), ids);
	}
}
