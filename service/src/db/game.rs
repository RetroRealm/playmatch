use crate::dat::shared::model;
use entity::sea_orm_active_enums::MatchTypeEnum;
use entity::{dat_file, dat_file_import, platform};
use ::entity::{
	game, game::Entity as Game, game_file, game_file::Entity as GameFile,
	signature_metadata_mapping,
};
use sea_orm::prelude::Uuid;
use sea_orm::sea_query::{Alias, Expr};
use sea_orm::{
	sea_query::SimpleExpr, ActiveEnum, ActiveModelTrait, ActiveValue::Set, ColumnTrait, DbConn,
	DbErr, EntityTrait, JoinType, Paginator, PaginatorTrait, QueryFilter, QuerySelect,
	RelationTrait, SelectModel, TryIntoModel,
};

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
		clone_of: Set(None),
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

pub async fn find_game_and_id_mapping_by_md5(
	md5: &str,
	conn: &DbConn,
) -> Result<Option<(game::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	find_signature_metadata_mapping_if_exists_by_filter(game_file::Column::Md5.eq(md5), conn).await
}

pub async fn find_game_and_id_mapping_by_sha1(
	sha1: &str,
	conn: &DbConn,
) -> Result<Option<(game::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	find_signature_metadata_mapping_if_exists_by_filter(game_file::Column::Sha1.eq(sha1), conn)
		.await
}

pub async fn find_game_and_id_mapping_by_sha256(
	sha256: &str,
	conn: &DbConn,
) -> Result<Option<(game::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	find_signature_metadata_mapping_if_exists_by_filter(game_file::Column::Sha256.eq(sha256), conn)
		.await
}

pub async fn find_game_and_id_mapping_by_name_and_size(
	name: &str,
	size: i64,
	conn: &DbConn,
) -> Result<Option<(game::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	find_signature_metadata_mapping_if_exists_by_filter(
		game_file::Column::FileName
			.eq(name)
			.and(game_file::Column::FileSizeInBytes.eq(size)),
		conn,
	)
	.await
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

pub fn get_unpopulated_clone_of_games(
	dat_file_id: Uuid,
	page_size: u64,
	conn: &DbConn,
) -> Paginator<DbConn, SelectModel<game::Model>> {
	Game::find()
		.filter(game::Column::SignatureGroupInternalCloneOfId.is_not_null())
		.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
		.filter(dat_file_import::Column::DatFileId.eq(dat_file_id))
		.paginate(conn, page_size)
}

pub fn get_unmatched_games_without_clone_of_id_paginator(
	page_size: u64,
	conn: &DbConn,
) -> Paginator<DbConn, SelectModel<game::Model>> {
	get_unmatched_games_paginator(true, page_size, conn)
}

pub fn get_unmatched_games_with_clone_of_id_paginator(
	page_size: u64,
	conn: &DbConn,
) -> Paginator<DbConn, SelectModel<game::Model>> {
	get_unmatched_games_paginator(false, page_size, conn)
}

fn get_unmatched_games_paginator(
	clone_of_null: bool,
	page_size: u64,
	conn: &DbConn,
) -> Paginator<DbConn, SelectModel<game::Model>> {
	let smm1 = Alias::new("smm1");
	let smm2 = Alias::new("smm2");

	Game::find()
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
		.filter(
			Expr::col((smm1.clone(), signature_metadata_mapping::Column::MatchType)).is_in(vec![
				MatchTypeEnum::Automatic.as_enum(),
				MatchTypeEnum::Manual.as_enum(),
			]),
		)
		.filter(if clone_of_null {
			game::Column::CloneOf.is_null()
		} else {
			game::Column::CloneOf.is_not_null()
		})
		.filter(
			Expr::col((smm2.clone(), signature_metadata_mapping::Column::Id))
				.is_null()
				.or(
					Expr::col((smm2, signature_metadata_mapping::Column::MatchType))
						.eq(MatchTypeEnum::None.as_enum()),
				),
		)
		.paginate(conn, page_size)
}
