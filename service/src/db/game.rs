use crate::dat::shared::model;
use crate::db::abstraction::ColumnEqIgnoreCaseTrait;
use ::entity::{
	game, game::Entity as Game, game_file, game_file::Entity as GameFile,
	signature_metadata_mapping,
};
use chrono::{Duration, NaiveDateTime, Utc};
use entity::sea_orm_active_enums::{FailedMatchReasonEnum, MatchTypeEnum};
use entity::{company, dat_file, dat_file_import, platform, signature_group};
use futures_util::future::BoxFuture;
use sea_orm::prelude::Uuid;
use sea_orm::sea_query::{Alias, Expr};
use sea_orm::{
	ActiveModelTrait, ActiveValue::Set, ColumnTrait, DbConn, DbErr, EntityTrait, JoinType,
	ModelTrait, Paginator, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, RelationTrait,
	SelectModel, TryIntoModel, sea_query::SimpleExpr,
};

pub async fn get_game_by_id(game_id: Uuid, conn: &DbConn) -> Result<Option<game::Model>, DbErr> {
	Game::find_by_id(game_id).one(conn).await
}

pub async fn find_all_relations_of_game(
	game: &game::Model,
	conn: &DbConn,
) -> Result<
	(
		dat_file_import::Model,
		dat_file::Model,
		signature_group::Model,
		platform::Model,
		Option<company::Model>,
		Vec<game_file::Model>,
	),
	DbErr,
> {
	let dat_file_import_opt = game.find_related(dat_file_import::Entity).one(conn).await?;

	let dat_file_import = dat_file_import_opt
		.ok_or_else(|| DbErr::RecordNotFound("Dat file import not found".to_string()))?;

	let dat_file_opt = dat_file_import
		.find_related(dat_file::Entity)
		.one(conn)
		.await?;

	let dat_file =
		dat_file_opt.ok_or_else(|| DbErr::RecordNotFound("Dat file not found".to_string()))?;

	let signature_group_opt = dat_file
		.find_related(signature_group::Entity)
		.one(conn)
		.await?;

	let signature_group = signature_group_opt
		.ok_or_else(|| DbErr::RecordNotFound("Signature group not found".to_string()))?;

	let platform_opt = dat_file.find_related(platform::Entity).one(conn).await?;

	let platform =
		platform_opt.ok_or_else(|| DbErr::RecordNotFound("Platform not found".to_string()))?;

	let company = platform.find_related(company::Entity).one(conn).await?;

	let game_files = game.find_related(game_file::Entity).all(conn).await?;

	Ok((
		dat_file_import,
		dat_file,
		signature_group,
		platform,
		company,
		game_files,
	))
}

pub async fn insert_game(
	dat_file_import_id: Uuid,
	game: model::Game,
	conn: &DbConn,
) -> Result<game::Model, DbErr> {
	let game = game::ActiveModel {
		dat_file_import_id: Set(dat_file_import_id),
		signature_group_internal_id: Set(game.id),
		signature_group_internal_clone_of_id: Set(game.cloneofid),
		name: Set(game.name),
		description: Set(game.description),
		categories: Set(game.category),
		..Default::default()
	};

	game.save(conn).await?.try_into_model()
}

pub async fn find_game_by_signature_group_internal_id_and_dat_file_id(
	signature_group_internal_id: String,
	dat_file_id: Uuid,
	conn: &DbConn,
) -> Result<Option<game::Model>, DbErr> {
	Game::find()
		.filter(game::Column::SignatureGroupInternalId.eq(signature_group_internal_id))
		.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
		.filter(dat_file_import::Column::DatFileId.eq(dat_file_id))
		.one(conn)
		.await
}

pub async fn find_game_by_name_and_dat_file_id(
	name: &str,
	dat_file_id: Uuid,
	conn: &DbConn,
) -> Result<Option<game::Model>, DbErr> {
	Game::find()
		.filter(game::Column::Name.eq(name))
		.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
		.filter(dat_file_import::Column::DatFileId.eq(dat_file_id))
		.one(conn)
		.await
}

pub async fn find_game_and_id_mapping_by_game_id(
	game_id: Uuid,
	conn: &DbConn,
) -> Result<Option<(game::Model, Option<signature_metadata_mapping::Model>)>, DbErr> {
	let game = Game::find()
		.filter(game::Column::Id.eq(game_id))
		.one(conn)
		.await?;

	let mapping = signature_metadata_mapping::Entity::find()
		.filter(signature_metadata_mapping::Column::GameId.eq(game_id))
		.one(conn)
		.await?;

	if let Some(game) = game {
		Ok(Some((game, mapping)))
	} else {
		Ok(None)
	}
}

pub async fn find_game_and_id_mapping_by_md5(
	md5: &str,
	conn: &DbConn,
) -> Result<Option<(game::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	find_signature_metadata_mapping_if_exists_by_filter(
		game_file::Column::Md5.eq_ignore_case(md5),
		conn,
	)
	.await
}

pub async fn find_game_and_id_mapping_by_sha1(
	sha1: &str,
	conn: &DbConn,
) -> Result<Option<(game::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	find_signature_metadata_mapping_if_exists_by_filter(
		game_file::Column::Sha1.eq_ignore_case(sha1),
		conn,
	)
	.await
}

pub async fn find_game_and_id_mapping_by_sha256(
	sha256: &str,
	conn: &DbConn,
) -> Result<Option<(game::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	find_signature_metadata_mapping_if_exists_by_filter(
		game_file::Column::Sha256.eq_ignore_case(sha256),
		conn,
	)
	.await
}

pub async fn find_game_and_id_mapping_by_name_and_size(
	name: &str,
	size: i64,
	conn: &DbConn,
) -> Result<Option<(game::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	find_signature_metadata_mapping_if_exists_by_filter(
		game_file::Column::FileName
			.eq_ignore_case(name)
			.and(game_file::Column::FileSizeInBytes.eq(size)),
		conn,
	)
	.await
}

pub async fn find_games_by_name_and_platform_id(
	name: &str,
	platform_id: Uuid,
	conn: &DbConn,
) -> Result<Vec<game::Model>, DbErr> {
	Game::find()
		.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
		.join(
			JoinType::InnerJoin,
			dat_file_import::Relation::DatFile.def(),
		)
		.join(JoinType::InnerJoin, dat_file::Relation::Platform.def())
		.filter(
			game::Column::Name
				.eq(name)
				.and(platform::Column::Id.eq(platform_id)),
		)
		.all(conn)
		.await
}

pub async fn find_game_by_name_or_game_file_name(
	name: &str,
	conn: &DbConn,
) -> Result<Option<game::Model>, DbErr> {
	let game_file = GameFile::find()
		.filter(game_file::Column::FileName.eq(name))
		.find_also_related(Game)
		.one(conn)
		.await?;

	if let Some((_, Some(game))) = game_file {
		return Ok(Some(game));
	}

	let game = Game::find()
		.filter(game::Column::Name.eq(name))
		.one(conn)
		.await?;

	match game {
		None => Ok(None),
		Some(game) => Ok(Some(game)),
	}
}

async fn find_signature_metadata_mapping_if_exists_by_filter(
	input: SimpleExpr,
	conn: &DbConn,
) -> Result<Option<(game::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	let game_file = GameFile::find()
		.filter(input)
		.find_also_related(Game)
		.one(conn)
		.await?;

	match game_file {
		Some((_, Some(game))) => {
			let signature_metadata_mappings = signature_metadata_mapping::Entity::find()
				.filter(signature_metadata_mapping::Column::GameId.eq(game.id))
				.all(conn)
				.await?;

			Ok(Some((game, signature_metadata_mappings)))
		}
		_ => Ok(None),
	}
}

pub async fn find_all_children_of_game(
	game: &game::Model,
	conn: &DbConn,
) -> Result<Vec<game::Model>, DbErr> {
	Game::find()
		.filter(game::Column::CloneOf.eq(game.id))
		.all(conn)
		.await
}

pub async fn find_game_parent(
	game: &game::Model,
	conn: &DbConn,
) -> Result<Option<game::Model>, DbErr> {
	match game.clone_of {
		Some(clone_of_id) => {
			Game::find()
				.filter(game::Column::Id.eq(clone_of_id))
				.one(conn)
				.await
		}
		None => Ok(None),
	}
}

pub async fn find_game_signature_metadata_mapping(
	game: &game::Model,
	conn: &DbConn,
) -> Result<Option<signature_metadata_mapping::Model>, DbErr> {
	signature_metadata_mapping::Entity::find()
		.filter(signature_metadata_mapping::Column::GameId.eq(game.id))
		.one(conn)
		.await
}

pub async fn get_dat_file_id_of_game(game: &game::Model, conn: &DbConn) -> Result<Uuid, DbErr> {
	let dat_file_import = dat_file_import::Entity::find()
		.filter(dat_file_import::Column::Id.eq(game.dat_file_import_id))
		.one(conn)
		.await?;

	match dat_file_import {
		Some(dat_file_import) => Ok(dat_file_import.dat_file_id),
		None => Err(DbErr::RecordNotFound(
			"Dat file import not found".to_string(),
		)),
	}
}
pub fn get_unpopulated_clone_of_games(
	dat_file_id: Uuid,
	page_size: u64,
	conn: &DbConn,
) -> Paginator<DbConn, SelectModel<game::Model>> {
	Game::find()
		.filter(
			game::Column::SignatureGroupInternalCloneOfId
				.is_not_null()
				.and(game::Column::CloneOf.is_null()),
		)
		.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
		.filter(dat_file_import::Column::DatFileId.eq(dat_file_id))
		.order_by_asc(game::Column::Id)
		.paginate(conn, page_size)
}

pub fn get_unmatched_games_without_clone_of_with_limit<'a>(
	page_size: u64,
	conn: DbConn,
) -> BoxFuture<'a, anyhow::Result<Option<Vec<game::Model>>>> {
	get_unmatched_games_with_limit(true, page_size, conn)
}

pub fn get_unmatched_games_with_clone_of_with_limit<'a>(
	page_size: u64,
	conn: DbConn,
) -> BoxFuture<'a, anyhow::Result<Option<Vec<game::Model>>>> {
	get_unmatched_games_with_limit(false, page_size, conn)
}

pub fn get_automatic_match_failed_games_with_limit<'a>(
	page_size: u64,
	conn: DbConn,
) -> BoxFuture<'a, anyhow::Result<Option<Vec<game::Model>>>> {
	Box::pin(async move {
		let sixty_days_ago = Utc::now() - Duration::days(60);
		let sixty_days_ago_naive: NaiveDateTime = sixty_days_ago.naive_utc();

		let res = Game::find()
			.join(
				JoinType::LeftJoin,
				game::Relation::SignatureMetadataMapping.def(),
			)
			.filter(
				signature_metadata_mapping::Column::MatchType
					.eq(MatchTypeEnum::Failed)
					.and(
						signature_metadata_mapping::Column::FailedMatchReason
							.eq(FailedMatchReasonEnum::NoDirectMatch),
					)
					.and(signature_metadata_mapping::Column::UpdatedAt.lt(sixty_days_ago_naive)),
			)
			.order_by_asc(game::Column::Id)
			.limit(page_size)
			.all(&conn)
			.await?;

		if res.is_empty() {
			Ok(None)
		} else {
			Ok(Some(res))
		}
	})
}

fn get_unmatched_games_with_limit<'a>(
	clone_of_null: bool,
	page_size: u64,
	conn: DbConn,
) -> BoxFuture<'a, anyhow::Result<Option<Vec<game::Model>>>> {
	Box::pin(async move {
		let smm1 = Alias::new("smm1");
		let smm2 = Alias::new("smm2");

		let res = Game::find()
			.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
			.join(
				JoinType::InnerJoin,
				dat_file_import::Relation::DatFile.def(),
			)
			.join(JoinType::InnerJoin, dat_file::Relation::Platform.def())
			.join_as(
				JoinType::InnerJoin,
				platform::Relation::SignatureMetadataMapping.def(),
				smm1.clone(),
			)
			.join_as(
				JoinType::LeftJoin,
				game::Relation::SignatureMetadataMapping.def(),
				smm2.clone(),
			)
			.filter(if clone_of_null {
				game::Column::CloneOf.is_null()
			} else {
				game::Column::CloneOf.is_not_null()
			})
			.filter(
				Expr::col((smm1.clone(), signature_metadata_mapping::Column::MatchType))
					.is_in(vec![MatchTypeEnum::Automatic, MatchTypeEnum::Manual]),
			)
			.filter(
				Expr::col((smm2.clone(), signature_metadata_mapping::Column::Id))
					.is_null()
					.or(
						Expr::col((smm2, signature_metadata_mapping::Column::MatchType))
							.eq(MatchTypeEnum::None),
					),
			)
			.order_by_asc(game::Column::Id)
			.limit(page_size)
			.all(&conn)
			.await?;

		if res.is_empty() {
			Ok(None)
		} else {
			Ok(Some(res))
		}
	})
}
