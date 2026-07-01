use crate::error;
use crate::model::bulk::{
	BatchValidation, BulkByIdResponse, BulkByIdResult, BulkByIdStatusV2, BulkByIdSummary,
	BulkIdsRequest, MAX_BULK_ITEMS, dedupe_ids, validate_batch_size,
};
use crate::routes::v2::error::{batch_too_large_response, v2_batch_error};
use actix_web::web::{Data, Json};
use actix_web::{HttpResponse, Responder, post};
use futures_util::stream::{self, StreamExt};
use sea_orm::DatabaseConnection;
use serde::Serialize;
use service::bulk::BULK_CONCURRENCY;
use service::db::game_file::get_game_file_by_id;
use service::entities::company::get_company_by_id_and_external_metadata;
use service::entities::dat_file::{DatFileDetail, find_dat_file_detail_by_id};
use service::entities::platform::get_platform_by_id_and_related_company_and_signature_metadata_mapping;
use service::entities::signature_group::find_signature_group_by_id;
use service::error::ServiceError;
use service::identification::get_game_by_id_from_db;
use service::model::{
	CompanyMetadataResponse, GameMetadataResponse, PlatformMetadataResponse, PlaymatchGameFile,
	PlaymatchGameFileV2, PlaymatchSignatureGroupV2,
};
use uuid::Uuid;

/// Shared bulk get-by-id pipeline. Gates the batch on size and emptiness, dedupes
/// ids while preserving first-occurrence order, then resolves each id with bounded
/// concurrency over the shared database pool. A missing id is a per-item
/// `not_found`, never a batch-level 404. The whole batch counts as one request
/// against the version scope's shared rate limiter.
async fn run_bulk_by_id<T, F, Fut>(
	endpoint: &str,
	ids: Vec<Uuid>,
	fetch: F,
) -> error::Result<HttpResponse>
where
	T: Serialize,
	F: Fn(Uuid) -> Fut + Copy,
	Fut: std::future::Future<Output = error::Result<Option<T>>>,
{
	match validate_batch_size(ids.len()) {
		BatchValidation::Empty => {
			return Ok(v2_batch_error(
				"empty_batch",
				"batch must contain at least one item",
				MAX_BULK_ITEMS,
				0,
			));
		}
		BatchValidation::TooLarge { received } => {
			return Ok(batch_too_large_response(MAX_BULK_ITEMS, received));
		}
		BatchValidation::Ok => {}
	}

	let ids = dedupe_ids(ids);
	service::metrics::observe_bulk_by_id_batch_size(endpoint, ids.len());

	let mut resolved: Vec<error::Result<(usize, Uuid, Option<T>)>> =
		stream::iter(ids.into_iter().enumerate())
			.map(|(index, id)| async move { fetch(id).await.map(|data| (index, id, data)) })
			.buffer_unordered(BULK_CONCURRENCY)
			.collect()
			.await;

	resolved.sort_by_key(|entry| {
		entry
			.as_ref()
			.map(|(index, _, _)| *index)
			.unwrap_or(usize::MAX)
	});

	let mut summary = BulkByIdSummary {
		total: resolved.len(),
		..Default::default()
	};
	let mut results = Vec::with_capacity(resolved.len());
	for entry in resolved {
		let (_, id, data) = entry?;
		let status = if data.is_some() {
			summary.found += 1;
			service::metrics::record_bulk_by_id_item(endpoint, "ok");
			BulkByIdStatusV2::Ok
		} else {
			summary.not_found += 1;
			service::metrics::record_bulk_by_id_item(endpoint, "not_found");
			BulkByIdStatusV2::NotFound
		};
		results.push(BulkByIdResult { id, status, data });
	}

	Ok(HttpResponse::Ok().json(BulkByIdResponse { summary, results }))
}

/// Map the game read's `GameNotFound` to a per-item `not_found`. The other three
/// resource reads already return `Ok(None)` for a miss, so only the game read
/// needs this adapter to fit the uniform `Option` shape.
async fn fetch_game(
	id: Uuid,
	db: &DatabaseConnection,
) -> error::Result<Option<GameMetadataResponse>> {
	match get_game_by_id_from_db(id, db).await {
		Ok(game) => Ok(Some(game)),
		Err(ServiceError::GameNotFound) => Ok(None),
		Err(other) => Err(other.into()),
	}
}

/// Looks up many games by id in one request.
///
/// Up to 100 ids per request; duplicate ids are resolved once. A missing id is a
/// per-item `not_found`, never a batch-level 404. The whole batch counts as one
/// request against the rate limiter.
#[utoipa::path(
	post,
	tag = "Game",
	request_body = BulkIdsRequest,
	responses(
		(status = 200, description = "Per-item game results with a batch summary", body = BulkGamesByIdResponse),
		(status = 400, description = "Empty batch, or batch over the 100-item cap", body = V2ErrorBody)
	)
)]
#[post("/games/bulk")]
pub async fn bulk_games_by_id_v2(
	body: Json<BulkIdsRequest>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let db = db_conn.get_ref();
	run_bulk_by_id("games", body.into_inner().ids, |id| fetch_game(id, db)).await
}

/// Looks up many platforms by id in one request.
///
/// Up to 100 ids per request; duplicate ids are resolved once. A missing id is a
/// per-item `not_found`, never a batch-level 404. The whole batch counts as one
/// request against the rate limiter.
#[utoipa::path(
	post,
	tag = "Platform",
	request_body = BulkIdsRequest,
	responses(
		(status = 200, description = "Per-item platform results with a batch summary", body = BulkPlatformsByIdResponse),
		(status = 400, description = "Empty batch, or batch over the 100-item cap", body = V2ErrorBody)
	)
)]
#[post("/platforms/bulk")]
pub async fn bulk_platforms_by_id_v2(
	body: Json<BulkIdsRequest>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let db = db_conn.get_ref();
	run_bulk_by_id::<PlatformMetadataResponse, _, _>(
		"platforms",
		body.into_inner().ids,
		|id| async move {
			Ok(
				get_platform_by_id_and_related_company_and_signature_metadata_mapping(id, db)
					.await?,
			)
		},
	)
	.await
}

/// Looks up many companies by id in one request.
///
/// Up to 100 ids per request; duplicate ids are resolved once. A missing id is a
/// per-item `not_found`, never a batch-level 404. The whole batch counts as one
/// request against the rate limiter.
#[utoipa::path(
	post,
	tag = "Company",
	request_body = BulkIdsRequest,
	responses(
		(status = 200, description = "Per-item company results with a batch summary", body = BulkCompaniesByIdResponse),
		(status = 400, description = "Empty batch, or batch over the 100-item cap", body = V2ErrorBody)
	)
)]
#[post("/companies/bulk")]
pub async fn bulk_companies_by_id_v2(
	body: Json<BulkIdsRequest>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let db = db_conn.get_ref();
	run_bulk_by_id::<CompanyMetadataResponse, _, _>(
		"companies",
		body.into_inner().ids,
		|id| async move { Ok(get_company_by_id_and_external_metadata(id, db).await?) },
	)
	.await
}

/// Looks up many dat files by id in one request.
///
/// Up to 100 ids per request; duplicate ids are resolved once. A missing id is a
/// per-item `not_found`, never a batch-level 404. The whole batch counts as one
/// request against the rate limiter.
#[utoipa::path(
	post,
	tag = "DAT File",
	request_body = BulkIdsRequest,
	responses(
		(status = 200, description = "Per-item dat file results with a batch summary", body = BulkDatFilesByIdResponse),
		(status = 400, description = "Empty batch, or batch over the 100-item cap", body = V2ErrorBody)
	)
)]
#[post("/dat-files/bulk")]
pub async fn bulk_dat_files_by_id_v2(
	body: Json<BulkIdsRequest>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let db = db_conn.get_ref();
	run_bulk_by_id::<DatFileDetail, _, _>("dat-files", body.into_inner().ids, |id| async move {
		Ok(find_dat_file_detail_by_id(id, db).await?)
	})
	.await
}

/// Looks up many signature groups by id in one request.
///
/// Up to 100 ids per request; duplicate ids are resolved once. A missing id is a
/// per-item `not_found`, never a batch-level 404. The whole batch counts as one
/// request against the rate limiter.
#[utoipa::path(
	post,
	tag = "Signature Group",
	request_body = BulkIdsRequest,
	responses(
		(status = 200, description = "Per-item signature group results with a batch summary", body = BulkSignatureGroupsByIdResponse),
		(status = 400, description = "Empty batch, or batch over the 100-item cap", body = V2ErrorBody)
	)
)]
#[post("/signature-groups/bulk")]
pub async fn bulk_signature_groups_by_id_v2(
	body: Json<BulkIdsRequest>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let db = db_conn.get_ref();
	run_bulk_by_id::<PlaymatchSignatureGroupV2, _, _>(
		"signature-groups",
		body.into_inner().ids,
		|id| async move {
			Ok(find_signature_group_by_id(id, db)
				.await?
				.map(PlaymatchSignatureGroupV2::from))
		},
	)
	.await
}

/// Looks up many game files by id in one request.
///
/// Up to 100 ids per request; duplicate ids are resolved once. A missing id is a
/// per-item `not_found`, never a batch-level 404. The whole batch counts as one
/// request against the rate limiter. Each file carries the same projection as
/// `/games/{id}/files`.
#[utoipa::path(
	post,
	tag = "Game",
	request_body = BulkIdsRequest,
	responses(
		(status = 200, description = "Per-item game file results with a batch summary", body = BulkGameFilesByIdResponse),
		(status = 400, description = "Empty batch, or batch over the 100-item cap", body = V2ErrorBody)
	)
)]
#[post("/game-files/bulk")]
pub async fn bulk_game_files_by_id_v2(
	body: Json<BulkIdsRequest>,
	db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
	let db = db_conn.get_ref();
	run_bulk_by_id::<PlaymatchGameFileV2, _, _>(
		"game-files",
		body.into_inner().ids,
		|id| async move {
			Ok(get_game_file_by_id(id, db)
				.await?
				.map(|file| PlaymatchGameFileV2::from(PlaymatchGameFile::from(file))))
		},
	)
	.await
}
