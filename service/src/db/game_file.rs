use crate::dat::shared::model::RomElement;
use entity::game_file;
use sea_orm::prelude::Uuid;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, DbConn};

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
