use crate::dat::shared::model::RomElement;
use entity::game_file;
use entity::game_file::ActiveModel;
use sea_orm::prelude::Uuid;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, DbConn, EntityTrait, QueryFilter};

pub async fn insert_game_file_bulk(
	game_files: Vec<RomElement>,
	game_id: Uuid,
	conn: &DbConn,
) -> anyhow::Result<()> {
	let mut to_insert = Vec::new();

	for game_file in game_files {
		let game_file = get_active_model_from_rom_element(game_id, game_file)?;

		to_insert.push(game_file);
	}

	game_file::Entity::insert_many(to_insert).exec(conn).await?;

	Ok(())
}

pub async fn insert_game_file(
	game_file: RomElement,
	game_id: Uuid,
	conn: &DbConn,
) -> anyhow::Result<ActiveModel> {
	let game_file = get_active_model_from_rom_element(game_id, game_file)?;

	game_file.save(conn).await.map_err(|e| e.into())
}

pub async fn get_game_files_from_game_id(
	game_id: Uuid,
	conn: &DbConn,
) -> anyhow::Result<Vec<game_file::Model>> {
	Ok(game_file::Entity::find()
		.filter(game_file::Column::GameId.eq(game_id))
		.all(conn)
		.await?)
}

fn get_active_model_from_rom_element(
	game_id: Uuid,
	game_file: RomElement,
) -> anyhow::Result<ActiveModel> {
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

	let game_file = ActiveModel {
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
	Ok(game_file)
}
