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

use crate::tools;

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
	/// Optional playmatch platform id as a UUID string to narrow the search.
	pub platform_id: Option<String>,
	/// Optional maximum number of candidates to return.
	pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GameIdArgs {
	/// The playmatch game id as a UUID string.
	pub game_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GameFileIdArgs {
	/// The playmatch game file id as a UUID string.
	pub game_file_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CompanyIdArgs {
	/// The playmatch company id as a UUID string.
	pub company_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PlatformIdArgs {
	/// The playmatch platform id as a UUID string.
	pub platform_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SignatureGroupIdArgs {
	/// The playmatch signature group id as a UUID string.
	pub signature_group_id: String,
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
	log::error!("mcp internal error in {context}: {err:?}");
	ErrorData::internal_error("internal error".to_string(), None)
}

fn invalid_uuid(raw: &str) -> CallToolResult {
	bad_input(format!("'{raw}' is not a valid UUID"))
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

	/// Identify a ROM and return its matched game id plus external metadata
	/// provider ids. Call this when you have a ROM file and want to know which
	/// game it is and how it maps to providers like IGDB. Provide hashes when
	/// available for the most reliable match. The gameMatchType field is a closed
	/// set: SHA256, SHA1, MD5, CRC, FileNameAndSize or NoMatch. NoMatch is a normal
	/// answer whose matched game id is null. FileNameAndSize is a weaker fallback
	/// that only matches when file_name is exactly the catalogued ROM name.
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

	/// Identify a ROM and return the matched game together with its related
	/// platform, company, signature group, dat file and game files. Call this
	/// when you need the full context around a ROM, not just its ids. The
	/// gameMatchType field is a closed set: SHA256, SHA1, MD5, CRC, FileNameAndSize or
	/// NoMatch. NoMatch is a normal answer whose matched game id is null.
	/// FileNameAndSize is a weaker fallback that only matches when file_name is
	/// exactly the catalogued ROM name.
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

	/// Find a playmatch game by its human title when you do not have a ROM file
	/// or any hash. This is a fuzzy substring search over the catalogue, ordered
	/// by relevance, returning candidate ids, names and platforms. Pass a returned
	/// id to playmatch_get_game or playmatch_get_game_with_relations for the full
	/// record. Narrow with platform_id when you know the platform, and cap the
	/// number of candidates with limit. An empty query is rejected.
	#[tool(
		description = "Find a playmatch game by human title when you do not have a hash. Fuzzy substring search over the catalogue, ordered by relevance, returning candidate ids, names and platforms. Pass a returned id to playmatch_get_game for the full record. Optional platform_id narrows to a platform; optional limit caps the candidates."
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

	/// Fetch a single game and its external metadata by its playmatch game id.
	/// Call this when you already have a game id and want its name, description
	/// and provider mappings. DAT-currency fields such as current_in_latest_dat
	/// are populated only by playmatch_get_game_with_relations.
	#[tool(
		description = "Fetch a single game and its external metadata by its playmatch game id. DAT-currency fields such as current_in_latest_dat are populated only by playmatch_get_game_with_relations."
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

	/// Fetch a game with its full relations (platform, company, signature group,
	/// dat file, dat file import and game files) by its playmatch game id.
	#[tool(
		description = "Fetch a game with its full relations (platform, company, signature group, dat file and files) by its playmatch game id."
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

	/// Return the dat file imports a game file was seen in, newest first. Call
	/// this with a game file id to trace which dat releases contained that file.
	#[tool(
		description = "Return the dat file imports a game file was seen in, newest first, by its playmatch game file id."
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

	/// List every company and its external metadata provider mappings. Call this
	/// to browse the full set of known publishers and developers.
	#[tool(description = "List every company and its external metadata provider mappings.")]
	async fn playmatch_list_companies(&self) -> Result<CallToolResult, ErrorData> {
		let json = tools::list_companies_json(&self.db)
			.await
			.map_err(|e| internal_error("list_companies", e))?;
		Ok(ok_text(json))
	}

	/// Fetch a single company and its external metadata by its playmatch company id.
	#[tool(
		description = "Fetch a single company and its external metadata by its playmatch company id."
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

	/// List every platform with its related company and external metadata
	/// provider mappings. Call this to browse the full set of known platforms.
	#[tool(
		description = "List every platform with its related company and external metadata provider mappings."
	)]
	async fn playmatch_list_platforms(&self) -> Result<CallToolResult, ErrorData> {
		let json = tools::list_platforms_json(&self.db)
			.await
			.map_err(|e| internal_error("list_platforms", e))?;
		Ok(ok_text(json))
	}

	/// Fetch a single platform with its related company and external metadata by
	/// its playmatch platform id.
	#[tool(
		description = "Fetch a single platform with its related company and external metadata by its playmatch platform id."
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

	/// List every signature group, the dat publishers playmatch ingests from
	/// such as No-Intro and Redump. Call this to browse the known signature groups.
	#[tool(description = "List every signature group (dat publisher) known to playmatch.")]
	async fn playmatch_list_signature_groups(&self) -> Result<CallToolResult, ErrorData> {
		let json = tools::list_signature_groups_json(&self.db)
			.await
			.map_err(|e| internal_error("list_signature_groups", e))?;
		Ok(ok_text(json))
	}

	/// Fetch a single signature group by its playmatch signature group id.
	#[tool(description = "Fetch a single signature group by its playmatch signature group id.")]
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
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for PlaymatchMcp {
	fn get_info(&self) -> ServerInfo {
		let mut info = ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
			.with_instructions(
				"playmatch identifies game ROMs by hash and exposes the playmatch catalogue. \
				 Use the identify tools to resolve a ROM file to a game; they accept SHA256, \
				 SHA1, MD5 and CRC hashes plus file name and size, and try them from most to \
				 least accurate. When you have no ROM and no hash, only a human title, use \
				 playmatch_search_games_by_name to find candidate game ids, then pass an id to \
				 playmatch_get_game or playmatch_get_game_with_relations. The get/list tools look \
				 up games, platforms, companies and signature groups by id. external_metadata \
				 entries are provider id mappings only; resolving them to full records needs the \
				 separate provider HTTP API, so the ids are references, not dead ends.",
			);
		info.server_info =
			Implementation::new("playmatch", env!("CARGO_PKG_VERSION")).with_title("Playmatch");
		info
	}
}
