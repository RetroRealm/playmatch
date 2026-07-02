use std::sync::Arc;

use redis::aio::MultiplexedConnection;
use rmcp::ErrorData;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo};
use rmcp::{ServerHandler, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use sea_orm::DatabaseConnection;
use sea_orm::prelude::Uuid;
use serde::Deserialize;
use service::entities::dat_file::HashLookup;

use crate::tools;
use crate::tools::{BulkIdentifyItem, MAX_BULK_ITEMS};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct IdentifyRomArgs {
	/// The file name of the ROM, including its extension.
	pub file_name: String,
	/// The size of the ROM file in bytes.
	pub file_size: i64,
	/// Optional MD5 hash of the ROM, lowercase hex, 32 characters.
	pub md5: Option<String>,
	/// Optional SHA1 hash of the ROM, lowercase hex, 40 characters.
	pub sha1: Option<String>,
	/// Optional SHA256 hash of the ROM, lowercase hex, 64 characters.
	pub sha256: Option<String>,
	/// Optional CRC32 checksum of the ROM, lowercase hex, 8 characters.
	pub crc: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchGamesArgs {
	/// The human game title to search for, for example "pokemon diamond".
	pub query: String,
	/// Optional Playmatch platform id as a UUID string to narrow the search.
	pub platform_id: Option<String>,
	/// Optional maximum number of candidates to return.
	pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GameIdArgs {
	/// The Playmatch game id as a UUID string.
	pub game_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GameFileIdArgs {
	/// The Playmatch game file id as a UUID string.
	pub game_file_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CompanyIdArgs {
	/// The Playmatch company id as a UUID string.
	pub company_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PlatformIdArgs {
	/// The Playmatch platform id as a UUID string.
	pub platform_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SignatureGroupIdArgs {
	/// The Playmatch signature group id as a UUID string.
	pub signature_group_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListDatFilesArgs {
	/// Optional signature group id as a UUID string to narrow the listing.
	pub signature_group_id: Option<String>,
	/// Optional platform id as a UUID string to narrow the listing.
	pub platform_id: Option<String>,
	/// Optional company id as a UUID string to narrow the listing.
	pub company_id: Option<String>,
	/// Optional case-insensitive substring to filter dat file names by.
	pub name: Option<String>,
	/// Optional maximum number of dat files to return.
	pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DatFileIdArgs {
	/// The Playmatch dat file id as a UUID string.
	pub dat_file_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListDatFileGamesArgs {
	/// The Playmatch dat file id as a UUID string.
	pub dat_file_id: String,
	/// Only include games present in the dat file's current release. Defaults to true.
	pub current_only: Option<bool>,
	/// Hydrate each game with its files. Defaults to false.
	pub include_files: Option<bool>,
	/// Hydrate each game with its external metadata mappings. Defaults to false.
	pub include_mappings: Option<bool>,
	/// Optional maximum number of games to return.
	pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GameFilesArgs {
	/// The Playmatch game id as a UUID string.
	pub game_id: String,
	/// Only include files present in the current release. Defaults to true.
	pub current_only: Option<bool>,
	/// Optional maximum number of files to return.
	pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct HashLookupArgs {
	/// Optional SHA256 hash, lowercase hex, 64 characters.
	pub sha256: Option<String>,
	/// Optional SHA1 hash, lowercase hex, 40 characters.
	pub sha1: Option<String>,
	/// Optional MD5 hash, lowercase hex, 32 characters.
	pub md5: Option<String>,
	/// Optional CRC32 checksum, lowercase hex, 8 characters.
	pub crc: Option<String>,
	/// Collapse results to signature groups instead of individual dat files. Defaults to false.
	pub as_groups: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BulkIdentifyArgs {
	/// The game files to identify, at most 100 per request.
	pub items: Vec<BulkIdentifyItemArgs>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BulkIdentifyItemArgs {
	/// The file name of the ROM, including its extension.
	pub file_name: String,
	/// The size of the ROM file in bytes.
	pub file_size: i64,
	/// Optional MD5 hash of the ROM, lowercase hex, 32 characters.
	pub md5: Option<String>,
	/// Optional SHA1 hash of the ROM, lowercase hex, 40 characters.
	pub sha1: Option<String>,
	/// Optional SHA256 hash of the ROM, lowercase hex, 64 characters.
	pub sha256: Option<String>,
	/// Optional CRC32 checksum of the ROM, lowercase hex, 8 characters.
	pub crc: Option<String>,
	/// Optional caller key, unique within the batch and at most 128 characters,
	/// echoed back on the matching result for correlation.
	pub key: Option<String>,
}

#[derive(Clone)]
pub struct PlaymatchMcp {
	db: Arc<DatabaseConnection>,
	redis: MultiplexedConnection,
	tool_router: ToolRouter<Self>,
}

fn ok_text(json: String) -> CallToolResult {
	CallToolResult::success(vec![Content::text(json)])
}

fn bad_input(message: impl Into<String>) -> CallToolResult {
	CallToolResult::error(vec![Content::text(message.into())])
}

fn not_found(kind: &str, id: &str) -> CallToolResult {
	bad_input(format!("no {kind} found with id '{id}'"))
}

fn internal_error(context: &str, err: anyhow::Error) -> ErrorData {
	log::error!("MCP internal error in {context}: {err:?}");
	ErrorData::internal_error("internal error".to_string(), None)
}

fn invalid_uuid(raw: &str) -> CallToolResult {
	bad_input(format!("'{raw}' is not a valid UUID"))
}

/// Parse an optional UUID argument. Returns the raw string on failure so the
/// caller can surface it through `invalid_uuid`, keeping the bulky
/// `CallToolResult` off the error path (see `clippy::result_large_err`).
fn parse_opt_uuid(raw: Option<&str>) -> Result<Option<Uuid>, &str> {
	match raw.filter(|s| !s.is_empty()) {
		Some(raw) => Uuid::parse_str(raw).map(Some).map_err(|_| raw),
		None => Ok(None),
	}
}

fn validate_hex(value: &Option<String>, expected_len: usize, label: &str) -> Result<(), String> {
	if let Some(v) = value.as_deref().filter(|s| !s.is_empty()) {
		if v.len() != expected_len {
			return Err(format!("{label} must be {expected_len} hex characters"));
		}
		if !v.chars().all(|c| c.is_ascii_hexdigit()) {
			return Err(format!("{label} must be hexadecimal"));
		}
	}
	Ok(())
}

#[tool_router]
impl PlaymatchMcp {
	pub fn new(db: Arc<DatabaseConnection>, redis: MultiplexedConnection) -> Self {
		Self {
			db,
			redis,
			tool_router: Self::tool_router(),
		}
	}

	#[tool(
		description = "Identify a ROM by its hashes and file metadata and return the matched game id and external metadata provider ids. The gameMatchType field is one of SHA256, SHA1, MD5, CRC, FileNameAndSize or NoMatch; NoMatch is a normal result whose matched game id is null. FileNameAndSize is a weaker fallback that only matches when file_name is the catalogued ROM name."
	)]
	async fn playmatch_identify_rom_by_hash(
		&self,
		Parameters(args): Parameters<IdentifyRomArgs>,
	) -> Result<CallToolResult, ErrorData> {
		let search = tools::build_search(
			args.file_name,
			args.file_size,
			args.md5,
			args.sha1,
			args.sha256,
			args.crc,
		);
		if let Err(e) = search.validate() {
			return Ok(bad_input(e));
		}
		let mut redis = self.redis.clone();
		let json = tools::identify_rom_by_hash_json(search, &mut redis, &self.db)
			.await
			.map_err(|e| internal_error("identify_rom_by_hash", e))?;
		Ok(ok_text(json))
	}

	#[tool(
		description = "Identify a ROM by its hashes and file metadata and return the matched game with its related platform, company, signature group, dat file and files. The gameMatchType field is one of SHA256, SHA1, MD5, CRC, FileNameAndSize or NoMatch; NoMatch is a normal result whose matched game id is null. FileNameAndSize is a weaker fallback that only matches when file_name is the catalogued ROM name."
	)]
	async fn playmatch_identify_rom_with_relations(
		&self,
		Parameters(args): Parameters<IdentifyRomArgs>,
	) -> Result<CallToolResult, ErrorData> {
		let search = tools::build_search(
			args.file_name,
			args.file_size,
			args.md5,
			args.sha1,
			args.sha256,
			args.crc,
		);
		if let Err(e) = search.validate() {
			return Ok(bad_input(e));
		}
		let mut redis = self.redis.clone();
		let json = tools::identify_rom_with_relations_json(search, &mut redis, &self.db)
			.await
			.map_err(|e| internal_error("identify_rom_with_relations", e))?;
		Ok(ok_text(json))
	}

	#[tool(
		description = "Find a Playmatch game by human title when you do not have a hash. Fuzzy substring search over the catalogue, ordered by relevance, returning candidate ids, names and platforms. Pass a returned id to playmatch_get_game for the full record. Optional platform_id narrows to a platform; optional limit caps the candidates."
	)]
	async fn playmatch_search_games_by_name(
		&self,
		Parameters(args): Parameters<SearchGamesArgs>,
	) -> Result<CallToolResult, ErrorData> {
		let query = args.query.trim();
		if query.is_empty() {
			return Ok(bad_input("query must not be empty"));
		}

		let platform_id = match args.platform_id.as_deref() {
			Some(raw) => match Uuid::parse_str(raw) {
				Ok(id) => Some(id),
				Err(_) => return Ok(invalid_uuid(raw)),
			},
			None => None,
		};

		let json = tools::search_games_by_name_json(
			query,
			platform_id,
			args.limit.map(u64::from),
			&self.db,
		)
		.await
		.map_err(|e| internal_error("search_games_by_name", e))?;
		Ok(ok_text(json))
	}

	#[tool(
		description = "Fetch a single game and its external metadata by its Playmatch game id. DAT-currency fields such as current_in_latest_dat are populated only by playmatch_get_game_with_relations."
	)]
	async fn playmatch_get_game(
		&self,
		Parameters(args): Parameters<GameIdArgs>,
	) -> Result<CallToolResult, ErrorData> {
		let Ok(game_id) = Uuid::parse_str(&args.game_id) else {
			return Ok(invalid_uuid(&args.game_id));
		};
		match tools::get_game_json(game_id, &self.db)
			.await
			.map_err(|e| internal_error("get_game", e))?
		{
			Some(json) => Ok(ok_text(json)),
			None => Ok(not_found("game", &args.game_id)),
		}
	}

	#[tool(
		description = "Fetch a game with its full relations (platform, company, signature group, dat file and files) by its Playmatch game id."
	)]
	async fn playmatch_get_game_with_relations(
		&self,
		Parameters(args): Parameters<GameIdArgs>,
	) -> Result<CallToolResult, ErrorData> {
		let Ok(game_id) = Uuid::parse_str(&args.game_id) else {
			return Ok(invalid_uuid(&args.game_id));
		};
		match tools::get_game_with_relations_json(game_id, &self.db)
			.await
			.map_err(|e| internal_error("get_game_with_relations", e))?
		{
			Some(json) => Ok(ok_text(json)),
			None => Ok(not_found("game", &args.game_id)),
		}
	}

	#[tool(
		description = "Return the dat file imports a game file was seen in, newest first, by its Playmatch game file id."
	)]
	async fn playmatch_get_game_file_history(
		&self,
		Parameters(args): Parameters<GameFileIdArgs>,
	) -> Result<CallToolResult, ErrorData> {
		let Ok(game_file_id) = Uuid::parse_str(&args.game_file_id) else {
			return Ok(invalid_uuid(&args.game_file_id));
		};
		let json = tools::get_game_file_history_json(game_file_id, &self.db)
			.await
			.map_err(|e| internal_error("get_game_file_history", e))?;
		Ok(ok_text(json))
	}

	#[tool(description = "List every company and its external metadata provider mappings.")]
	async fn playmatch_list_companies(&self) -> Result<CallToolResult, ErrorData> {
		let json = tools::list_companies_json(&self.db)
			.await
			.map_err(|e| internal_error("list_companies", e))?;
		Ok(ok_text(json))
	}

	#[tool(
		description = "Fetch a single company and its external metadata by its Playmatch company id."
	)]
	async fn playmatch_get_company(
		&self,
		Parameters(args): Parameters<CompanyIdArgs>,
	) -> Result<CallToolResult, ErrorData> {
		let Ok(company_id) = Uuid::parse_str(&args.company_id) else {
			return Ok(invalid_uuid(&args.company_id));
		};
		match tools::get_company_json(company_id, &self.db)
			.await
			.map_err(|e| internal_error("get_company", e))?
		{
			Some(json) => Ok(ok_text(json)),
			None => Ok(not_found("company", &args.company_id)),
		}
	}

	#[tool(
		description = "List every platform with its related company and external metadata provider mappings."
	)]
	async fn playmatch_list_platforms(&self) -> Result<CallToolResult, ErrorData> {
		let json = tools::list_platforms_json(&self.db)
			.await
			.map_err(|e| internal_error("list_platforms", e))?;
		Ok(ok_text(json))
	}

	#[tool(
		description = "Fetch a single platform with its related company and external metadata by its Playmatch platform id."
	)]
	async fn playmatch_get_platform(
		&self,
		Parameters(args): Parameters<PlatformIdArgs>,
	) -> Result<CallToolResult, ErrorData> {
		let Ok(platform_id) = Uuid::parse_str(&args.platform_id) else {
			return Ok(invalid_uuid(&args.platform_id));
		};
		match tools::get_platform_json(platform_id, &self.db)
			.await
			.map_err(|e| internal_error("get_platform", e))?
		{
			Some(json) => Ok(ok_text(json)),
			None => Ok(not_found("platform", &args.platform_id)),
		}
	}

	#[tool(description = "List every signature group (dat publisher) known to Playmatch.")]
	async fn playmatch_list_signature_groups(&self) -> Result<CallToolResult, ErrorData> {
		let json = tools::list_signature_groups_json(&self.db)
			.await
			.map_err(|e| internal_error("list_signature_groups", e))?;
		Ok(ok_text(json))
	}

	#[tool(description = "Fetch a single signature group by its Playmatch signature group id.")]
	async fn playmatch_get_signature_group(
		&self,
		Parameters(args): Parameters<SignatureGroupIdArgs>,
	) -> Result<CallToolResult, ErrorData> {
		let Ok(signature_group_id) = Uuid::parse_str(&args.signature_group_id) else {
			return Ok(invalid_uuid(&args.signature_group_id));
		};
		match tools::get_signature_group_json(signature_group_id, &self.db)
			.await
			.map_err(|e| internal_error("get_signature_group", e))?
		{
			Some(json) => Ok(ok_text(json)),
			None => Ok(not_found("signature group", &args.signature_group_id)),
		}
	}

	#[tool(
		description = "List dat files in the catalogue, newest catalogue ordering, each with its signature group, platform, optional company, current version and latest import. Narrow with optional signature_group_id, platform_id, company_id and a case-insensitive name substring; cap with limit. Pass a returned id to playmatch_get_dat_file or playmatch_list_dat_file_games."
	)]
	async fn playmatch_list_dat_files(
		&self,
		Parameters(args): Parameters<ListDatFilesArgs>,
	) -> Result<CallToolResult, ErrorData> {
		let signature_group_id = match parse_opt_uuid(args.signature_group_id.as_deref()) {
			Ok(id) => id,
			Err(raw) => return Ok(invalid_uuid(raw)),
		};
		let platform_id = match parse_opt_uuid(args.platform_id.as_deref()) {
			Ok(id) => id,
			Err(raw) => return Ok(invalid_uuid(raw)),
		};
		let company_id = match parse_opt_uuid(args.company_id.as_deref()) {
			Ok(id) => id,
			Err(raw) => return Ok(invalid_uuid(raw)),
		};

		let json = tools::list_dat_files_json(
			signature_group_id,
			platform_id,
			company_id,
			args.name,
			args.limit.map(u64::from),
			&self.db,
		)
		.await
		.map_err(|e| internal_error("list_dat_files", e))?;
		Ok(ok_text(json))
	}

	#[tool(
		description = "Fetch a single dat file with its related entities and aggregate game counts by its Playmatch dat file id."
	)]
	async fn playmatch_get_dat_file(
		&self,
		Parameters(args): Parameters<DatFileIdArgs>,
	) -> Result<CallToolResult, ErrorData> {
		let Ok(dat_file_id) = Uuid::parse_str(&args.dat_file_id) else {
			return Ok(invalid_uuid(&args.dat_file_id));
		};
		match tools::get_dat_file_json(dat_file_id, &self.db)
			.await
			.map_err(|e| internal_error("get_dat_file", e))?
		{
			Some(json) => Ok(ok_text(json)),
			None => Ok(not_found("dat file", &args.dat_file_id)),
		}
	}

	#[tool(
		description = "List the games in a dat file by its Playmatch dat file id. Defaults to current games only; set current_only false for every game ever seen. Opt into per-game files with include_files and external metadata mappings with include_mappings; cap with limit."
	)]
	async fn playmatch_list_dat_file_games(
		&self,
		Parameters(args): Parameters<ListDatFileGamesArgs>,
	) -> Result<CallToolResult, ErrorData> {
		let Ok(dat_file_id) = Uuid::parse_str(&args.dat_file_id) else {
			return Ok(invalid_uuid(&args.dat_file_id));
		};
		match tools::list_dat_file_games_json(
			dat_file_id,
			args.current_only.unwrap_or(true),
			args.include_files.unwrap_or(false),
			args.include_mappings.unwrap_or(false),
			args.limit.map(u64::from),
			&self.db,
		)
		.await
		.map_err(|e| internal_error("list_dat_file_games", e))?
		{
			Some(json) => Ok(ok_text(json)),
			None => Ok(not_found("dat file", &args.dat_file_id)),
		}
	}

	#[tool(
		description = "List the catalogued files (ROM dumps with their hashes) of a game by its Playmatch game id. Defaults to current files only; set current_only false for every file ever seen. Cap with limit."
	)]
	async fn playmatch_get_game_files(
		&self,
		Parameters(args): Parameters<GameFilesArgs>,
	) -> Result<CallToolResult, ErrorData> {
		let Ok(game_id) = Uuid::parse_str(&args.game_id) else {
			return Ok(invalid_uuid(&args.game_id));
		};
		match tools::get_game_files_json(
			game_id,
			args.current_only.unwrap_or(true),
			args.limit.map(u64::from),
			&self.db,
		)
		.await
		.map_err(|e| internal_error("get_game_files", e))?
		{
			Some(json) => Ok(ok_text(json)),
			None => Ok(not_found("game", &args.game_id)),
		}
	}

	#[tool(
		description = "Reverse hash lookup: find the dat files (or signature groups, with as_groups true) whose imports contain the file matched by the supplied hashes. The strongest supplied hash resolves the file (sha256 > sha1 > md5 > crc), matching the identify cascade. Each entry carries the first and last import the file was seen in and whether it is current in the dat file's latest release. At least one hash is required; returns not found when no file matches."
	)]
	async fn playmatch_find_dats_containing_hash(
		&self,
		Parameters(args): Parameters<HashLookupArgs>,
	) -> Result<CallToolResult, ErrorData> {
		if let Err(message) = validate_hex(&args.sha256, 64, "sha256")
			.and_then(|()| validate_hex(&args.sha1, 40, "sha1"))
			.and_then(|()| validate_hex(&args.md5, 32, "md5"))
			.and_then(|()| validate_hex(&args.crc, 8, "crc"))
		{
			return Ok(bad_input(message));
		}

		let lookup = HashLookup {
			sha256: args.sha256.filter(|s| !s.is_empty()),
			sha1: args.sha1.filter(|s| !s.is_empty()),
			md5: args.md5.filter(|s| !s.is_empty()),
			crc: args.crc.filter(|s| !s.is_empty()),
		};
		if lookup.is_empty() {
			return Ok(bad_input(
				"at least one of sha256, sha1, md5 or crc is required",
			));
		}

		match tools::find_dats_containing_hash_json(
			lookup,
			args.as_groups.unwrap_or(false),
			&self.db,
		)
		.await
		.map_err(|e| internal_error("find_dats_containing_hash", e))?
		{
			Some(json) => Ok(ok_text(json)),
			None => Ok(bad_input("no file matches any supplied hash")),
		}
	}

	#[tool(
		description = "Identify a batch of game files by hash or name+size in one call, returning a per-item result with a summary. At most 100 items per request; each item may carry an optional key (unique within the batch) echoed back for correlation. Per-item status is ok (cascade ran, match may be NoMatch), invalid (failed validation) or error (per-item failure); the batch still succeeds. Mirrors playmatch_identify_rom_by_hash per item."
	)]
	async fn playmatch_bulk_identify(
		&self,
		Parameters(args): Parameters<BulkIdentifyArgs>,
	) -> Result<CallToolResult, ErrorData> {
		if args.items.is_empty() {
			return Ok(bad_input("items must not be empty"));
		}
		if args.items.len() > MAX_BULK_ITEMS {
			return Ok(bad_input(format!(
				"batch exceeds the {MAX_BULK_ITEMS}-item cap"
			)));
		}

		let mut seen = std::collections::HashSet::new();
		for (index, item) in args.items.iter().enumerate() {
			let Some(key) = item.key.as_deref() else {
				continue;
			};
			if key.chars().count() > 128 {
				return Ok(bad_input(format!(
					"item {index}: key exceeds 128 characters"
				)));
			}
			if !seen.insert(key) {
				return Ok(bad_input(format!(
					"item {index}: key is not unique within the batch"
				)));
			}
		}

		let items = args
			.items
			.into_iter()
			.map(|item| BulkIdentifyItem {
				search: tools::build_search(
					item.file_name,
					item.file_size,
					item.md5,
					item.sha1,
					item.sha256,
					item.crc,
				),
				key: item.key,
			})
			.collect();

		let json = tools::bulk_identify_json(items, &self.redis, &self.db)
			.await
			.map_err(|e| internal_error("bulk_identify", e))?;
		Ok(ok_text(json))
	}
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for PlaymatchMcp {
	fn get_info(&self) -> ServerInfo {
		let mut info = ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
			.with_instructions(
				"Playmatch identifies game ROMs by hash and exposes the Playmatch catalogue. \
				 Use the identify tools to resolve a ROM file to a game; they accept SHA256, \
				 SHA1, MD5 and CRC hashes plus file name and size, and try them from most to \
				 least accurate. When you have no ROM and no hash, only a human title, use \
				 playmatch_search_games_by_name to find candidate game ids, then pass an id to \
				 playmatch_get_game or playmatch_get_game_with_relations. The get/list tools look \
				 up games, platforms, companies and signature groups by id, browse dat files with \
				 playmatch_list_dat_files and their games and files with playmatch_list_dat_file_games \
				 and playmatch_get_game_files. playmatch_find_dats_containing_hash is the reverse \
				 lookup from a hash to the dat files that carry it, and playmatch_bulk_identify runs \
				 up to 100 identify cascades in one call. external_metadata entries are provider id \
				 mappings only; resolving them to full records needs the separate provider HTTP API, \
				 so the ids are references, not dead ends.",
			);
		info.server_info =
			Implementation::new("playmatch", env!("CARGO_PKG_VERSION")).with_title("Playmatch");
		info
	}
}
