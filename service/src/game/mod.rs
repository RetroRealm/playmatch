use crate::cache::identify::{
	find_game_and_id_mapping_by_md5_cached, find_game_and_id_mapping_by_sha1_cached,
	find_game_and_id_mapping_by_sha256_cached,
};
use crate::db::game::{
	find_all_relations_of_game, find_game_and_id_mapping_by_name_and_size, get_game_by_id,
};
use crate::error::{ServiceError, ServiceResult};
use crate::manual_match::build_result;
use crate::model::{
	GameAndRelationMatchResult, GameAndRelationMatchResultBuilder, GameAndRelationsResult,
	GameAndRelationsResultBuilder, GameFileMatchSearch, GameMatchType, GameMetadataMatchResult,
	PlaymatchGame,
};
use sea_orm::DbConn;
use sea_orm::prelude::Uuid;
use strum::IntoEnumIterator;

pub async fn get_game_by_id_from_db(game_id: Uuid, conn: &DbConn) -> ServiceResult<PlaymatchGame> {
	let game_opt = get_game_by_id(game_id, conn).await?;

	let game = game_opt.ok_or(ServiceError::GameNotFound)?;

	Ok(game.into())
}

pub async fn get_game_and_all_relations(
	game_id: Uuid,
	conn: &DbConn,
) -> ServiceResult<GameAndRelationsResult> {
	let game_opt = get_game_by_id(game_id, conn).await?;

	let game = game_opt.ok_or(ServiceError::GameNotFound)?;

	let (dat_file_import, dat_file, signature_group, platform, company, game_files) =
		find_all_relations_of_game(&game, conn).await?;

	Ok(GameAndRelationsResultBuilder::default()
		.game(game.into())
		.platform(platform.into())
		.company(company.map(|c| c.into()))
		.game_files(game_files.into_iter().map(|gf| gf.into()).collect())
		.dat_file(dat_file.into())
		.dat_file_import(dat_file_import.into())
		.signature_group(signature_group.into())
		.build()?)
}

pub async fn identify_game_and_get_relations(
	search: GameFileMatchSearch,
	conn: &DbConn,
) -> anyhow::Result<GameAndRelationMatchResult> {
	let mut response_body = None;

	for r#type in GameMatchType::iter() {
		if r#type == GameMatchType::NoMatch {
			continue;
		}

		if let Some((game_release, metadata_mappings)) = match r#type {
			GameMatchType::SHA256 => {
				if let Some(sha256) = &search.sha256 {
					find_game_and_id_mapping_by_sha256_cached(sha256, conn).await?
				} else {
					None
				}
			}
			GameMatchType::SHA1 => {
				if let Some(sha1) = &search.sha1 {
					find_game_and_id_mapping_by_sha1_cached(sha1, conn).await?
				} else {
					None
				}
			}
			GameMatchType::MD5 => {
				if let Some(md5) = &search.md5 {
					find_game_and_id_mapping_by_md5_cached(md5, conn).await?
				} else {
					None
				}
			}
			GameMatchType::FileNameAndSize => {
				find_game_and_id_mapping_by_name_and_size(&search.file_name, search.file_size, conn)
					.await?
			}
			GameMatchType::NoMatch => unreachable!(),
		} {
			let (dat_file_import, dat_file, signature_group, platform, company, game_files) =
				find_all_relations_of_game(&game_release, conn).await?;

			response_body = Some(
				GameAndRelationMatchResultBuilder::default()
					.game_match_type(r#type)
					.game(Some(game_release.into()))
					.platform(Some(platform.into()))
					.company(company.map(|c| c.into()))
					.game_files(game_files.into_iter().map(|gf| gf.into()).collect())
					.dat_file(Some(dat_file.into()))
					.dat_file_import(Some(dat_file_import.into()))
					.signature_group(Some(signature_group.into()))
					.external_metadata(
						metadata_mappings
							.into_iter()
							.map(|mapping| mapping.into())
							.collect(),
					)
					.build()?,
			);

			break;
		}
	}

	Ok(response_body.unwrap_or(GameAndRelationMatchResult {
		game_match_type: GameMatchType::NoMatch,
		game: None,
		game_files: vec![],
		company: None,
		platform: None,
		dat_file_import: None,
		dat_file: None,
		signature_group: None,
		external_metadata: vec![],
	}))
}

pub async fn identify_game_and_metadata_mappings(
	search: GameFileMatchSearch,
	conn: &DbConn,
) -> anyhow::Result<GameMetadataMatchResult> {
	let mut response_body = None;

	for r#type in GameMatchType::iter() {
		if r#type == GameMatchType::NoMatch {
			continue;
		}

		if let Some((game_release, game_release_id_mappings)) = match r#type {
			GameMatchType::SHA256 => {
				if let Some(sha256) = &search.sha256 {
					find_game_and_id_mapping_by_sha256_cached(sha256, conn).await?
				} else {
					None
				}
			}
			GameMatchType::SHA1 => {
				if let Some(sha1) = &search.sha1 {
					find_game_and_id_mapping_by_sha1_cached(sha1, conn).await?
				} else {
					None
				}
			}
			GameMatchType::MD5 => {
				if let Some(md5) = &search.md5 {
					find_game_and_id_mapping_by_md5_cached(md5, conn).await?
				} else {
					None
				}
			}
			GameMatchType::FileNameAndSize => {
				find_game_and_id_mapping_by_name_and_size(&search.file_name, search.file_size, conn)
					.await?
			}
			GameMatchType::NoMatch => unreachable!(),
		} {
			response_body = Some(build_result(
				r#type,
				game_release,
				game_release_id_mappings,
			)?);
			break;
		}
	}

	Ok(response_body.unwrap_or(GameMetadataMatchResult {
		game_match_type: GameMatchType::NoMatch,
		id: None,
		external_metadata: Vec::new(),
	}))
}
