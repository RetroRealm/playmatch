use futures_util::stream::{self, StreamExt};
use redis::aio::MultiplexedConnection;
use sea_orm::DatabaseConnection;
use sea_orm::prelude::Uuid;
use serde::Serialize;
use service::bulk::BULK_CONCURRENCY;
pub use service::bulk::MAX_BULK_ITEMS;
use service::cache::CacheStatus;
use service::db::dat_file::DatFileBrowseFilters;
use service::entities::dat_file::{
	DatFileGameHydration, HashLookup, find_dat_file_detail_by_id, find_dat_file_games_page,
	find_dat_file_presence_for_hash_lookup, find_dat_files_page,
};
use service::entities::game::find_game_files_page_for_game;
use service::error::ServiceError;
use service::identification::identify_game_and_metadata_mappings;
use service::model::{GameFileMatchSearch, GameMatchType, GameMetadataMatchResult};

fn unwrap_cache<T>(status: CacheStatus<T>) -> T {
	match status {
		CacheStatus::Cached(value) | CacheStatus::NonCached(value) => value,
	}
}

pub fn build_search(
	file_name: String,
	file_size: i64,
	md5: Option<String>,
	sha1: Option<String>,
	sha256: Option<String>,
	crc: Option<String>,
) -> GameFileMatchSearch {
	GameFileMatchSearch {
		file_name,
		file_size,
		md5,
		sha1,
		sha256,
		crc,
	}
}

pub async fn identify_rom_by_hash_json(
	search: GameFileMatchSearch,
	redis: &mut MultiplexedConnection,
	db: &DatabaseConnection,
) -> anyhow::Result<String> {
	let result =
		service::identification::identify_game_and_metadata_mappings(search, redis, db).await?;
	Ok(serde_json::to_string(&unwrap_cache(result))?)
}

pub async fn identify_rom_with_relations_json(
	search: GameFileMatchSearch,
	redis: &mut MultiplexedConnection,
	db: &DatabaseConnection,
) -> anyhow::Result<String> {
	let result =
		service::identification::identify_game_and_get_relations(search, redis, db).await?;
	Ok(serde_json::to_string(&unwrap_cache(result))?)
}

pub async fn search_games_by_name_json(
	query: &str,
	platform_id: Option<Uuid>,
	limit: Option<u64>,
	db: &DatabaseConnection,
) -> anyhow::Result<String> {
	let results =
		service::entities::game::search_games_by_name_and_platform(query, platform_id, limit, db)
			.await?;
	Ok(serde_json::to_string(&results)?)
}

pub async fn get_game_json(
	game_id: Uuid,
	db: &DatabaseConnection,
) -> anyhow::Result<Option<String>> {
	match service::identification::get_game_by_id_from_db(game_id, db).await {
		Ok(game) => Ok(Some(serde_json::to_string(&game)?)),
		Err(ServiceError::GameNotFound) => Ok(None),
		Err(e) => Err(e.into()),
	}
}

pub async fn get_game_with_relations_json(
	game_id: Uuid,
	db: &DatabaseConnection,
) -> anyhow::Result<Option<String>> {
	match service::identification::get_game_and_all_relations(game_id, db).await {
		Ok(game) => Ok(Some(serde_json::to_string(&game)?)),
		Err(ServiceError::GameNotFound) => Ok(None),
		Err(e) => Err(e.into()),
	}
}

pub async fn get_game_file_history_json(
	game_file_id: Uuid,
	db: &DatabaseConnection,
) -> anyhow::Result<String> {
	let history = service::identification::get_game_file_history(game_file_id, db).await?;
	Ok(serde_json::to_string(&history)?)
}

pub async fn list_companies_json(db: &DatabaseConnection) -> anyhow::Result<String> {
	let companies =
		service::entities::company::find_all_companies_and_external_metadata(db).await?;
	Ok(serde_json::to_string(&companies)?)
}

pub async fn get_company_json(
	company_id: Uuid,
	db: &DatabaseConnection,
) -> anyhow::Result<Option<String>> {
	let company =
		service::entities::company::get_company_by_id_and_external_metadata(company_id, db).await?;
	company
		.map(|c| serde_json::to_string(&c))
		.transpose()
		.map_err(Into::into)
}

pub async fn list_platforms_json(db: &DatabaseConnection) -> anyhow::Result<String> {
	let platforms =
		service::entities::platform::find_all_and_related_company_and_signature_metadata_mapping(
			db,
		)
		.await?;
	Ok(serde_json::to_string(&platforms)?)
}

pub async fn get_platform_json(
	platform_id: Uuid,
	db: &DatabaseConnection,
) -> anyhow::Result<Option<String>> {
	let platform =
		service::entities::platform::get_platform_by_id_and_related_company_and_signature_metadata_mapping(
			platform_id,
			db,
		)
		.await?;
	platform
		.map(|p| serde_json::to_string(&p))
		.transpose()
		.map_err(Into::into)
}

pub async fn list_signature_groups_json(db: &DatabaseConnection) -> anyhow::Result<String> {
	let groups = service::entities::signature_group::find_all_signature_groups(db).await?;
	Ok(serde_json::to_string(&groups)?)
}

pub async fn get_signature_group_json(
	signature_group_id: Uuid,
	db: &DatabaseConnection,
) -> anyhow::Result<Option<String>> {
	let group =
		service::entities::signature_group::find_signature_group_by_id(signature_group_id, db)
			.await?;
	group
		.map(|g| serde_json::to_string(&g))
		.transpose()
		.map_err(Into::into)
}

pub async fn list_dat_files_json(
	signature_group_id: Option<Uuid>,
	platform_id: Option<Uuid>,
	company_id: Option<Uuid>,
	name_contains: Option<String>,
	limit: Option<u64>,
	db: &DatabaseConnection,
) -> anyhow::Result<String> {
	let filters = DatFileBrowseFilters {
		signature_group_id,
		platform_id,
		company_id,
		subset: None,
		tag: None,
		name_contains: name_contains.filter(|s| !s.is_empty()),
	};
	let page = find_dat_files_page(&filters, None, limit, db).await?;
	Ok(serde_json::to_string(&page.rows)?)
}

pub async fn get_dat_file_json(
	dat_file_id: Uuid,
	db: &DatabaseConnection,
) -> anyhow::Result<Option<String>> {
	find_dat_file_detail_by_id(dat_file_id, db)
		.await?
		.map(|detail| serde_json::to_string(&detail))
		.transpose()
		.map_err(Into::into)
}

pub async fn list_dat_file_games_json(
	dat_file_id: Uuid,
	current_only: bool,
	include_files: bool,
	include_mappings: bool,
	limit: Option<u64>,
	db: &DatabaseConnection,
) -> anyhow::Result<Option<String>> {
	let hydration = DatFileGameHydration {
		include_files,
		include_mappings,
	};
	find_dat_file_games_page(dat_file_id, current_only, hydration, None, limit, db)
		.await?
		.map(|page| serde_json::to_string(&page.rows))
		.transpose()
		.map_err(Into::into)
}

pub async fn get_game_files_json(
	game_id: Uuid,
	current_only: bool,
	limit: Option<u64>,
	db: &DatabaseConnection,
) -> anyhow::Result<Option<String>> {
	find_game_files_page_for_game(game_id, current_only, None, limit, db)
		.await?
		.map(|page| serde_json::to_string(&page.rows))
		.transpose()
		.map_err(Into::into)
}

pub async fn find_dats_containing_hash_json(
	lookup: HashLookup,
	as_groups: bool,
	db: &DatabaseConnection,
) -> anyhow::Result<Option<String>> {
	find_dat_file_presence_for_hash_lookup(&lookup, as_groups, db)
		.await?
		.map(|result| serde_json::to_string(&result))
		.transpose()
		.map_err(Into::into)
}

#[derive(Serialize)]
struct BulkIdentifyResult {
	index: usize,
	#[serde(skip_serializing_if = "Option::is_none")]
	key: Option<String>,
	status: &'static str,
	#[serde(skip_serializing_if = "Option::is_none")]
	cache: Option<&'static str>,
	#[serde(rename = "match", skip_serializing_if = "Option::is_none")]
	game_match: Option<GameMetadataMatchResult>,
	#[serde(skip_serializing_if = "Option::is_none")]
	error: Option<String>,
}

#[derive(Serialize, Default)]
struct BulkIdentifySummary {
	total: usize,
	succeeded: usize,
	failed: usize,
	matched: usize,
	unmatched: usize,
}

#[derive(Serialize)]
struct BulkIdentifyResponse {
	summary: BulkIdentifySummary,
	results: Vec<BulkIdentifyResult>,
}

enum ItemOutcome {
	Ok {
		cache: &'static str,
		matched: bool,
		result: GameMetadataMatchResult,
	},
	Invalid(String),
	Error,
}

/// One item of a bulk identify request: the file search plus an optional caller
/// key echoed back for correlation. Validation and key uniqueness are enforced
/// by the caller before this is processed.
pub struct BulkIdentifyItem {
	pub search: GameFileMatchSearch,
	pub key: Option<String>,
}

async fn run_bulk_item(
	search: GameFileMatchSearch,
	mut redis: MultiplexedConnection,
	db: &DatabaseConnection,
) -> ItemOutcome {
	if let Err(msg) = search.validate() {
		return ItemOutcome::Invalid(msg);
	}
	match identify_game_and_metadata_mappings(search, &mut redis, db).await {
		Ok(CacheStatus::Cached(result)) => ItemOutcome::Ok {
			cache: "HIT",
			matched: result.game_match_type != GameMatchType::NoMatch,
			result,
		},
		Ok(CacheStatus::NonCached(result)) => ItemOutcome::Ok {
			cache: "MISS",
			matched: result.game_match_type != GameMatchType::NoMatch,
			result,
		},
		Err(_) => ItemOutcome::Error,
	}
}

/// Identify a batch of game files in one call, reusing the single-item cascade so
/// matching cannot diverge. Items run with bounded concurrency over the shared
/// Redis connection and database pool; a per-item failure never fails the batch.
/// The caller enforces the size cap and key uniqueness before invoking this.
pub async fn bulk_identify_json(
	items: Vec<BulkIdentifyItem>,
	redis: &MultiplexedConnection,
	db: &DatabaseConnection,
) -> anyhow::Result<String> {
	service::metrics::observe_bulk_identify_batch_size("mcp", items.len());

	let mut outcomes: Vec<(usize, Option<String>, ItemOutcome)> =
		stream::iter(items.into_iter().enumerate())
			.map(|(index, item)| {
				let redis = redis.clone();
				async move {
					let outcome = run_bulk_item(item.search, redis, db).await;
					(index, item.key, outcome)
				}
			})
			.buffer_unordered(BULK_CONCURRENCY)
			.collect()
			.await;

	outcomes.sort_by_key(|(index, _, _)| *index);

	let mut summary = BulkIdentifySummary {
		total: outcomes.len(),
		..Default::default()
	};
	let mut results = Vec::with_capacity(outcomes.len());
	for (index, key, outcome) in outcomes {
		let result = match outcome {
			ItemOutcome::Ok {
				cache,
				matched,
				result,
			} => {
				summary.succeeded += 1;
				if matched {
					summary.matched += 1;
				} else {
					summary.unmatched += 1;
				}
				service::metrics::record_bulk_identify_item("ok");
				BulkIdentifyResult {
					index,
					key,
					status: "ok",
					cache: Some(cache),
					game_match: Some(result),
					error: None,
				}
			}
			ItemOutcome::Invalid(message) => {
				summary.failed += 1;
				service::metrics::record_bulk_identify_item("invalid");
				BulkIdentifyResult {
					index,
					key,
					status: "invalid",
					cache: None,
					game_match: None,
					error: Some(message),
				}
			}
			ItemOutcome::Error => {
				summary.failed += 1;
				service::metrics::record_bulk_identify_item("error");
				BulkIdentifyResult {
					index,
					key,
					status: "error",
					cache: None,
					game_match: None,
					error: Some("identification failed for this item".to_string()),
				}
			}
		};
		results.push(result);
	}

	Ok(serde_json::to_string(&BulkIdentifyResponse {
		summary,
		results,
	})?)
}

#[cfg(test)]
mod tests {
	use super::build_search;
	use crate::server::SearchGamesArgs;
	use sea_orm::prelude::Uuid;

	#[test]
	fn invalid_uuid_is_rejected() {
		let parsed = Uuid::parse_str("not-a-uuid");
		assert!(parsed.is_err());
	}

	#[test]
	fn search_args_deserialize_with_optional_fields_absent() {
		let args: SearchGamesArgs = serde_json::from_str(r#"{"query":"pokemon diamond"}"#).unwrap();
		assert_eq!(args.query, "pokemon diamond");
		assert!(args.platform_id.is_none());
		assert!(args.limit.is_none());
	}

	#[test]
	fn search_args_blank_query_trims_to_empty() {
		let args: SearchGamesArgs = serde_json::from_str(r#"{"query":"   "}"#).unwrap();
		assert!(
			args.query.trim().is_empty(),
			"a whitespace-only query must trim to empty so the tool can reject it"
		);
	}

	#[test]
	fn search_args_platform_id_uuid_is_validated() {
		let args: SearchGamesArgs =
			serde_json::from_str(r#"{"query":"zelda","platform_id":"not-a-uuid"}"#).unwrap();
		let raw = args.platform_id.expect("platform_id present");
		assert!(
			Uuid::parse_str(&raw).is_err(),
			"a malformed platform_id must fail uuid parsing"
		);
	}

	#[test]
	fn search_validation_rejects_negative_file_size() {
		let search = build_search("game.rom".to_string(), -1, None, None, None, None);
		let err = search.validate().expect_err("negative size must fail");
		assert!(err.contains("file_size"));
	}

	#[test]
	fn search_validation_rejects_malformed_hash() {
		let search = build_search(
			"game.rom".to_string(),
			1024,
			Some("zzzz".to_string()),
			None,
			None,
			None,
		);
		assert!(search.validate().is_err());
	}

	#[test]
	fn search_validation_accepts_valid_input() {
		let search = build_search(
			"game.rom".to_string(),
			1024,
			Some("d41d8cd98f00b204e9800998ecf8427e".to_string()),
			None,
			None,
			None,
		);
		assert!(search.validate().is_ok());
	}

	#[test]
	fn search_validation_accepts_valid_crc() {
		let search = build_search(
			"game.rom".to_string(),
			1024,
			None,
			None,
			None,
			Some("1a2b3c4d".to_string()),
		);
		assert!(search.validate().is_ok());
	}

	#[test]
	fn search_validation_rejects_malformed_crc() {
		let search = build_search(
			"game.rom".to_string(),
			1024,
			None,
			None,
			None,
			Some("1a2b3c".to_string()),
		);
		let err = search.validate().expect_err("short crc must fail");
		assert!(err.contains("crc"));
	}

	#[test]
	fn list_dat_file_games_args_default_to_current_only_without_hydration() {
		use crate::server::ListDatFileGamesArgs;
		let args: ListDatFileGamesArgs =
			serde_json::from_str(r#"{"dat_file_id":"a1a1a1a1-a1a1-a1a1-a1a1-a1a1a1a1a1a1"}"#)
				.unwrap();
		assert!(
			args.current_only.is_none(),
			"current_only defaults at the tool layer"
		);
		assert!(args.include_files.is_none());
		assert!(args.include_mappings.is_none());
		assert!(args.limit.is_none());
	}

	#[test]
	fn hash_lookup_args_accept_a_partial_hash_set() {
		use crate::server::HashLookupArgs;
		let args: HashLookupArgs =
			serde_json::from_str(r#"{"crc":"1a2b3c4d","as_groups":true}"#).unwrap();
		assert_eq!(args.crc.as_deref(), Some("1a2b3c4d"));
		assert!(args.sha256.is_none());
		assert_eq!(args.as_groups, Some(true));
	}

	#[test]
	fn bulk_identify_args_carry_items_and_optional_keys() {
		use crate::server::BulkIdentifyArgs;
		let args: BulkIdentifyArgs = serde_json::from_str(
			r#"{"items":[{"file_name":"a.rom","file_size":1,"key":"k1"},{"file_name":"b.rom","file_size":2}]}"#,
		)
		.unwrap();
		assert_eq!(args.items.len(), 2);
		assert_eq!(args.items[0].key.as_deref(), Some("k1"));
		assert!(args.items[1].key.is_none());
	}
}
