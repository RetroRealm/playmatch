use crate::db::dat_file::find_all_dat_files;
use crate::db::game::{
	find_game_by_signature_group_internal_id_and_dat_file_id, get_dat_file_id_of_game,
	get_unpopulated_clone_of_games,
};
use crate::r#match::PAGE_SIZE;
use entity::game;
use log::debug;
use sea_orm::prelude::Uuid;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, DbConn, IntoActiveModel};
use tokio::task::JoinHandle;

pub async fn populate_all_clone_of_ids(conn: &DbConn) -> anyhow::Result<()> {
	let dat_files = find_all_dat_files(conn).await?;

	for dat_file in dat_files {
		populate_clone_of_id(dat_file.id, conn).await?;
	}

	Ok(())
}

pub async fn populate_clone_of_id(dat_file_id: Uuid, conn: &DbConn) -> anyhow::Result<()> {
	let mut paginator = get_unpopulated_clone_of_games(dat_file_id, PAGE_SIZE, conn);

	while let Some(games_to_match) = paginator.fetch_and_next().await? {
		for games_chunk in games_to_match.chunks(25).map(|x| x.to_vec()) {
			let mut futures: Vec<JoinHandle<anyhow::Result<()>>> = vec![];
			for game in games_chunk {
				let conn = conn.clone();
				futures.push(tokio::spawn(
					async move { try_match_parent(game, &conn).await },
				));
			}

			for future in futures {
				future.await??;
			}
		}
	}

	debug!("Created all clone_of relationships for games with internal_clone_of_id for dat_file_id: {}", dat_file_id);

	Ok(())
}

async fn try_match_parent(game: game::Model, conn: &DbConn) -> anyhow::Result<()> {
	if let Some(signature_group_internal_clone_of_id) = &game.signature_group_internal_clone_of_id {
		let dat_file_id = get_dat_file_id_of_game(&game, conn).await?;

		let game_parent = find_game_by_signature_group_internal_id_and_dat_file_id(
			signature_group_internal_clone_of_id.clone(),
			dat_file_id,
			conn,
		)
		.await?;

		if let Some(game_parent) = game_parent {
			let mut game_active_model = game.into_active_model();

			game_active_model.clone_of = Set(Some(game_parent.id));

			game_active_model.save(conn).await?;
		}
	}

	Ok(())
}
