use sea_orm::prelude::Uuid;
use sea_orm::{
	sea_query::SimpleExpr, ActiveModelTrait, ActiveValue::Set, ColumnTrait, DbConn, DbErr,
	EntityTrait, JoinType, QueryFilter, QuerySelect, RelationTrait,
};

use crate::dat::shared::model::{Game, RomElement};
use entity::{dat_file, dat_file_import};
use ::entity::{
	game, game::Entity as GameRelease, game_file, game_file::Entity as GameFile,
	signature_metadata_mapping,
};

pub async fn insert_game_file(
	game_file: RomElement,
	game_id: Uuid,
	conn: &DbConn,
) -> anyhow::Result<game_file::ActiveModel> {
	let file_size = match game_file.size {
		None => None,
		Some(inner) => {
			if inner.is_empty() {
				None
			} else {
				Some(inner.parse::<i64>()?)
			}
		}
	};

	let game_file = game_file::ActiveModel {
		file_name: Set(game_file.name),
		file_size_in_bytes: Set(file_size),
		crc: Set(game_file.crc),
		md5: Set(game_file.md5),
		sha1: Set(game_file.sha1),
		sha256: Set(game_file.sha256),
		status: Set(game_file.status.map(|s| s.to_string())),
		serial: Set(game_file.serial),
		game_id: Set(game_id),
		..Default::default()
	};

	game_file.save(conn).await.map_err(|e| e.into())
}

pub async fn insert_game(
	dat_file_import_id: Uuid,
	game: Game,
	conn: &DbConn,
) -> Result<game::ActiveModel, DbErr> {
	let original_game_id = match game.cloneofid {
		None => None,
		Some(clone_of_id) => {
			GameRelease::find()
				.filter(game::Column::SignatureGroupInternalId.eq(clone_of_id))
				.one(conn)
				.await?
		}
	}
	.map(|game| game.id);

	let game = game::ActiveModel {
		dat_file_import_id: Set(dat_file_import_id),
		signature_group_internal_id: Set(game.id),
		name: Set(game.name),
		description: Set(game.description),
		categories: Set(game.category),
		clone_of: Set(original_game_id),
		..Default::default()
	};

	game.save(conn).await
}

pub async fn find_game_by_name_and_platform_and_platform_company(
	name: &str,
	company_id: Option<Uuid>,
	platform_id: Uuid,
	conn: &DbConn,
) -> Result<Option<game::Model>, DbErr> {
	GameRelease::find()
		.filter(game::Column::Name.eq(name))
		.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
		.join(
			JoinType::InnerJoin,
			dat_file_import::Relation::DatFile.def(),
		)
		.filter(dat_file::Column::PlatformId.eq(platform_id))
		.filter(dat_file::Column::CompanyId.eq(company_id))
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
		.find_also_related(GameRelease)
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
