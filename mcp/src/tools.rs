use redis::aio::MultiplexedConnection;
use sea_orm::DatabaseConnection;
use sea_orm::prelude::Uuid;
use service::cache::CacheStatus;
use service::error::ServiceError;
use service::model::GameFileMatchSearch;

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

#[cfg(test)]
mod tests {
	use super::build_search;
	use sea_orm::prelude::Uuid;

	#[test]
	fn invalid_uuid_is_rejected() {
		let parsed = Uuid::parse_str("not-a-uuid");
		assert!(parsed.is_err());
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
}
