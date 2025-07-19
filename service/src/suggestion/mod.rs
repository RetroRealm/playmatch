use crate::db::company::find_company_by_name;
use crate::db::game::{
	find_game_and_id_mapping_by_md5, find_game_and_id_mapping_by_sha1,
	find_game_and_id_mapping_by_sha256, find_game_by_name_or_game_file_name,
};
use crate::db::platform::find_platform_by_name;
use crate::db::signature_metadata_mapping::{
	SignatureMetadataMappingInputBuilder, create_or_update_signature_metadata_mapping,
};
use crate::db::signature_metadata_mapping_suggestions::{
	get_all_suggestions, get_suggestion_by_id, insert_suggestion, suggestion_exists,
};
use crate::error::{ServiceError, ServiceResult};
use crate::manual_match::apply_manual_game_match_by_game;
use crate::model::ManualMatchMode;
use crate::model::matching::GameMatchData;
use crate::model::suggestion::{
	CompanyOrPlatformSuggestionRequest, GameSuggestionRequest, Suggestion,
};
use entity::sea_orm_active_enums::{ManualMatchModeEnum, MatchTypeEnum};
use sea_orm::prelude::Uuid;
use sea_orm::{DatabaseConnection, ModelTrait, Set};

pub async fn get_suggestions(db_conn: &DatabaseConnection) -> ServiceResult<Vec<Suggestion>> {
	let suggestions = get_all_suggestions(db_conn).await?;

	Ok(suggestions.into_iter().map(|s| s.into()).collect())
}

pub async fn get_suggestion(id: Uuid, db_conn: &DatabaseConnection) -> ServiceResult<Suggestion> {
	let suggestion = get_suggestion_by_id(id, db_conn)
		.await?
		.ok_or(ServiceError::SuggestionNotFound)?;

	Ok(suggestion.into())
}

pub async fn add_game_suggestion(
	request: GameSuggestionRequest,
	conn: &DatabaseConnection,
) -> ServiceResult<Suggestion> {
	let found_game = if let Some(sha256) = &request.sha256 {
		find_game_and_id_mapping_by_sha256(sha256, conn)
			.await?
			.map(|(game, _)| game)
	} else if let Some(sha1) = &request.sha1 {
		find_game_and_id_mapping_by_sha1(sha1, conn)
			.await?
			.map(|(game, _)| game)
	} else if let Some(md5) = &request.md5 {
		find_game_and_id_mapping_by_md5(md5, conn)
			.await?
			.map(|(game, _)| game)
	} else if let Some(file_name) = &request.name {
		find_game_by_name_or_game_file_name(file_name, conn).await?
	} else {
		None
	};

	let game = found_game.ok_or(ServiceError::GameNotFound)?;

	let suggestion = entity::signature_metadata_mapping_suggestions::ActiveModel {
		game_id: Set(Some(game.id)),
		provider: Set(request.provider.into()),
		provider_id: Set(request.provider_id.clone()),
		comment: Set(request.comment),
		created_by: Set(request.user_id),
		..Default::default()
	};

	let already_exists = suggestion_exists(
		Some(game.id),
		None,
		None,
		request.provider.into(),
		request.provider_id,
		conn,
	)
	.await?;

	if already_exists {
		return Err(ServiceError::SuggestionAlreadyExists);
	}

	let created = insert_suggestion(suggestion, conn).await?;

	Ok(created.into())
}

pub async fn add_platform_suggestion(
	request: CompanyOrPlatformSuggestionRequest,
	conn: &DatabaseConnection,
) -> ServiceResult<Suggestion> {
	let found_platform = find_platform_by_name(request.name.as_str(), conn).await?;

	let platform = found_platform.ok_or(ServiceError::PlatformNotFound)?;

	let suggestion = entity::signature_metadata_mapping_suggestions::ActiveModel {
		platform_id: Set(Some(platform.id)),
		provider: Set(request.provider.into()),
		provider_id: Set(request.provider_id.clone()),
		comment: Set(request.comment),
		created_by: Set(request.user_id),
		..Default::default()
	};

	let already_exists = suggestion_exists(
		None,
		Some(platform.id),
		None,
		request.provider.into(),
		request.provider_id,
		conn,
	)
	.await?;

	if already_exists {
		return Err(ServiceError::SuggestionAlreadyExists);
	}

	let created = insert_suggestion(suggestion, conn).await?;

	Ok(created.into())
}

pub async fn add_company_suggestion(
	request: CompanyOrPlatformSuggestionRequest,
	conn: &DatabaseConnection,
) -> ServiceResult<Suggestion> {
	let found_company = find_company_by_name(request.name.as_str(), conn).await?;

	let company = found_company.ok_or(ServiceError::CompanyNotFound)?;

	let suggestion = entity::signature_metadata_mapping_suggestions::ActiveModel {
		company_id: Set(Some(company.id)),
		provider: Set(request.provider.into()),
		provider_id: Set(request.provider_id.clone()),
		comment: Set(request.comment),
		created_by: Set(request.user_id),
		..Default::default()
	};

	let already_exists = suggestion_exists(
		None,
		None,
		Some(company.id),
		request.provider.into(),
		request.provider_id,
		conn,
	)
	.await?;

	if already_exists {
		return Err(ServiceError::SuggestionAlreadyExists);
	}

	let created = insert_suggestion(suggestion, conn).await?;

	Ok(created.into())
}

pub async fn accept_suggestion(id: Uuid, db_conn: &DatabaseConnection) -> ServiceResult<i32> {
	let suggestion_opt = get_suggestion_by_id(id, db_conn).await?;

	let suggestion = suggestion_opt.ok_or(ServiceError::SuggestionNotFound)?;

	let updated = if let Some(game_id) = suggestion.game_id {
		let game_opt = crate::db::game::get_game_by_id(game_id, db_conn).await?;

		let game = game_opt.ok_or(ServiceError::GameNotFound)?;

		let updated = apply_manual_game_match_by_game(
			game,
			GameMatchData {
				comment: suggestion.comment.clone(),
				provider: suggestion.provider.into(),
				provider_id: suggestion.provider_id.clone(),
				manual_match_type: ManualMatchMode::Community,
				user_id: suggestion.created_by,
			},
			db_conn,
		)
		.await?;

		updated.len() as i32
	} else {
		create_or_update_signature_metadata_mapping(
			SignatureMetadataMappingInputBuilder::default()
				.provider(suggestion.provider)
				.provider_id(Some(suggestion.provider_id.clone()))
				.game_id(suggestion.game_id)
				.platform_id(suggestion.platform_id)
				.company_id(suggestion.company_id)
				.comment(suggestion.comment.clone())
				.manually_matched_by(suggestion.created_by)
				.manual_match_type(Some(ManualMatchModeEnum::Community))
				.match_type(MatchTypeEnum::Manual)
				.failed_match_reason(None)
				.automatic_match_reason(None)
				.build()?,
			db_conn,
		)
		.await?;

		1
	};

	suggestion.delete(db_conn).await?;

	Ok(updated)
}

pub async fn decline_suggestion(id: Uuid, db_conn: &DatabaseConnection) -> ServiceResult<()> {
	let suggestion_opt = get_suggestion_by_id(id, db_conn).await?;

	let suggestion = suggestion_opt.ok_or(ServiceError::SuggestionNotFound)?;

	suggestion.delete(db_conn).await?;

	Ok(())
}
