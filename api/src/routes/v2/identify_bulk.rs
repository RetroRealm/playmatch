use crate::error;
use crate::model::bulk::{
	BatchValidation, BulkCache, BulkIdentifyItem, BulkIdentifyRequest, BulkIdentifyResponse,
	BulkIdentifyResult, BulkIdentifySummary, BulkItemError, BulkItemStatus, MAX_BULK_ITEMS,
	validate_batch_size, validate_keys,
};
use crate::routes::v2::error::{batch_too_large_response, v2_bad_request, v2_batch_error};
use actix_web::web::{Data, Json};
use actix_web::{HttpResponse, Responder, post};
use futures_util::stream::{self, StreamExt};
use redis::aio::MultiplexedConnection;
use sea_orm::DatabaseConnection;
use serde::Serialize;
use service::bulk::BULK_CONCURRENCY;
use service::cache::CacheStatus;
use service::identification::{
	identify_game_and_get_relations, identify_game_and_metadata_mappings,
};
use service::model::{
	GameAndRelationMatchResult, GameAndRelationMatchResultV2, GameMatchType,
	GameMetadataMatchResult,
};

/// Extracts whether an identify result is an actual match. A no-match still
/// counts as a successful (`ok`) cascade run; this only feeds the matched vs
/// unmatched summary tally.
trait Matched {
	fn matched(&self) -> bool;
}

macro_rules! impl_matched {
	($($result:ty),+ $(,)?) => {$(
		impl Matched for $result {
			fn matched(&self) -> bool {
				self.game_match_type != GameMatchType::NoMatch
			}
		}
	)+};
}

impl_matched!(
	GameMetadataMatchResult,
	GameAndRelationMatchResult,
	GameAndRelationMatchResultV2,
);

/// Maps a batch-level key-validation error code back to its static form for the
/// v2 envelope, whose `code` is `&'static str`. `validate_keys` only ever yields
/// these two codes; an unexpected one falls back to a generic invalid-key code.
fn bulk_item_error_code(code: &str) -> &'static str {
	match code {
		"key_too_long" => "key_too_long",
		"duplicate_key" => "duplicate_key",
		_ => "invalid_key",
	}
}

/// Outcome of processing a single item, before it is folded into the response
/// envelope and the summary counters.
enum ItemOutcome<T> {
	Ok { cache: BulkCache, result: T },
	Invalid(BulkItemError),
	Error(BulkItemError),
}

async fn run_item<T, F, Fut>(item: &BulkIdentifyItem, run: F) -> ItemOutcome<T>
where
	F: Fn(service::model::GameFileMatchSearch) -> Fut,
	Fut: std::future::Future<Output = anyhow::Result<CacheStatus<T>>>,
{
	if let Err(msg) = item.search.validate() {
		return ItemOutcome::Invalid(BulkItemError {
			code: "invalid_item".to_string(),
			field: None,
			message: msg,
		});
	}

	match run(item.search.clone()).await {
		Ok(CacheStatus::Cached(result)) => ItemOutcome::Ok {
			cache: BulkCache::Hit,
			result,
		},
		Ok(CacheStatus::NonCached(result)) => ItemOutcome::Ok {
			cache: BulkCache::Miss,
			result,
		},
		Err(err) => {
			// A per-item infra failure (Redis/DB) is otherwise invisible beyond the
			// aggregate error counter; log the cause so a batch-wide outage is
			// diagnosable without exposing internals to the client.
			log::warn!("bulk identify item failed: {err:?}");
			ItemOutcome::Error(BulkItemError {
				code: "identify_failed".to_string(),
				field: None,
				message: "identification failed for this item".to_string(),
			})
		}
	}
}

fn assemble_response<T: Serialize + Matched>(
	outcomes: Vec<(usize, Option<String>, ItemOutcome<T>)>,
) -> HttpResponse {
	let mut summary = BulkIdentifySummary {
		total: outcomes.len(),
		..Default::default()
	};
	let mut results = Vec::with_capacity(outcomes.len());
	let mut cache_hits = 0usize;
	let mut cache_misses = 0usize;

	for (index, key, outcome) in outcomes {
		let result = match outcome {
			ItemOutcome::Ok { cache, result } => {
				summary.succeeded += 1;
				if result.matched() {
					summary.matched += 1;
				} else {
					summary.unmatched += 1;
				}
				match cache {
					BulkCache::Hit => cache_hits += 1,
					BulkCache::Miss => cache_misses += 1,
				}
				service::metrics::record_bulk_identify_item("ok");
				BulkIdentifyResult {
					index,
					key,
					status: BulkItemStatus::Ok,
					cache: Some(cache),
					r#match: Some(result),
					error: None,
				}
			}
			ItemOutcome::Invalid(err) => {
				summary.failed += 1;
				service::metrics::record_bulk_identify_item("invalid");
				BulkIdentifyResult {
					index,
					key,
					status: BulkItemStatus::Invalid,
					cache: None,
					r#match: None,
					error: Some(err),
				}
			}
			ItemOutcome::Error(err) => {
				summary.failed += 1;
				service::metrics::record_bulk_identify_item("error");
				BulkIdentifyResult {
					index,
					key,
					status: BulkItemStatus::Error,
					cache: None,
					r#match: None,
					error: Some(err),
				}
			}
		};
		results.push(result);
	}

	// Bulk drops the single endpoint's per-response X-Cache in favour of an
	// aggregate. Tokens stay UPPERCASE to match the single endpoint's vocabulary.
	HttpResponse::Ok()
		.append_header((
			"X-Cache-Summary",
			format!(
				"{}={cache_hits};{}={cache_misses}",
				BulkCache::Hit.as_str(),
				BulkCache::Miss.as_str()
			),
		))
		.json(BulkIdentifyResponse { summary, results })
}

/// Shared bulk pipeline. Gates the batch, then runs the per-item cascade with
/// bounded concurrency over the shared Redis and database handles. Returns the
/// envelope ordered by request index. A per-item infra failure does not fail the
/// batch; the batch only short-circuits on the envelope gate (size, emptiness,
/// key collisions).
async fn run_bulk<T, F, Fut>(
	endpoint: &str,
	body: BulkIdentifyRequest,
	redis_conn: &MultiplexedConnection,
	run: F,
) -> HttpResponse
where
	T: Serialize + Matched,
	F: Fn(service::model::GameFileMatchSearch, MultiplexedConnection) -> Fut + Copy,
	Fut: std::future::Future<Output = anyhow::Result<CacheStatus<T>>>,
{
	let items = body.items;

	match validate_batch_size(items.len()) {
		BatchValidation::Empty => {
			return v2_batch_error(
				"empty_batch",
				"batch must contain at least one item",
				MAX_BULK_ITEMS,
				0,
			);
		}
		BatchValidation::TooLarge { received } => {
			return batch_too_large_response(MAX_BULK_ITEMS, received);
		}
		BatchValidation::Ok => {}
	}

	if let Err((index, err)) = validate_keys(&items) {
		return v2_bad_request(
			bulk_item_error_code(&err.code),
			format!("item {index}: {}", err.message),
		);
	}

	service::metrics::observe_bulk_identify_batch_size(endpoint, items.len());

	let mut outcomes: Vec<(usize, Option<String>, ItemOutcome<T>)> =
		stream::iter(items.into_iter().enumerate())
			.map(|(index, item)| {
				let redis = redis_conn.clone();
				async move {
					let outcome = run_item(&item, |search| run(search, redis.clone())).await;
					(index, item.key, outcome)
				}
			})
			.buffer_unordered(BULK_CONCURRENCY)
			.collect()
			.await;

	outcomes.sort_by_key(|(index, _, _)| *index);

	assemble_response(outcomes)
}

/// Identifies many game files in one request, returning metadata mappings.
///
/// Each item is resolved by its file hashes or by filename and size. Up to 100
/// items per request. A per-item failure does not fail the batch; it is reported
/// on that item. The whole batch counts as one request against the rate limiter.
#[utoipa::path(
	post,
	tag = "Identify",
	request_body = BulkIdentifyRequest,
	responses(
		(status = 200, description = "Per-item identify results with a batch summary", body = BulkIdentifyIdsResponse),
		(status = 400, description = "Empty batch, batch over the 100-item cap, or malformed body", body = V2ErrorBody)
	)
)]
#[post("/identify/bulk/ids")]
pub async fn identify_bulk_ids_v2(
	body: Json<BulkIdentifyRequest>,
	db_conn: Data<DatabaseConnection>,
	redis_conn: Data<MultiplexedConnection>,
) -> error::Result<impl Responder> {
	let db = db_conn.get_ref();
	let redis = redis_conn.get_ref();
	Ok(run_bulk(
		"ids",
		body.into_inner(),
		redis,
		|search, mut conn| async move {
			identify_game_and_metadata_mappings(search, &mut conn, db).await
		},
	)
	.await)
}

/// Identifies many game files in one request, returning full relations.
///
/// Each item is resolved by its file hashes or by filename and size, with the
/// matched game's relations on each result. Up to 100 items per request. A per-item
/// failure does not fail the batch; it is reported on that item. The whole batch
/// counts as one request against the rate limiter.
///
/// `additionalMatches` is omitted on every item here to keep the batch cheap.
/// Resolve the co-hashed siblings for a specific file through the single
/// `/identify` or `/identify/relations` endpoint.
#[utoipa::path(
	post,
	tag = "Identify",
	request_body = BulkIdentifyRequest,
	responses(
		(status = 200, description = "Per-item identify results with a batch summary", body = BulkIdentifyRelationsResponse),
		(status = 400, description = "Empty batch, batch over the 100-item cap, or malformed body", body = V2ErrorBody)
	)
)]
#[post("/identify/bulk/relations")]
pub async fn identify_bulk_relations_v2(
	body: Json<BulkIdentifyRequest>,
	db_conn: Data<DatabaseConnection>,
	redis_conn: Data<MultiplexedConnection>,
) -> error::Result<impl Responder> {
	let db = db_conn.get_ref();
	let redis = redis_conn.get_ref();
	Ok(run_bulk(
		"relations",
		body.into_inner(),
		redis,
		|search, mut conn| async move {
			let status = identify_game_and_get_relations(search, &mut conn, db).await?;
			Ok(match status {
				CacheStatus::Cached(result) => {
					CacheStatus::Cached(GameAndRelationMatchResultV2::from(result))
				}
				CacheStatus::NonCached(result) => {
					CacheStatus::NonCached(GameAndRelationMatchResultV2::from(result))
				}
			})
		},
	)
	.await)
}
