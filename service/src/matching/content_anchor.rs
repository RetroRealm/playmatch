use crate::config::PARALLELISM;
use crate::db::game_file::{assign_content_anchor_for_game, get_games_without_content_anchor};
use entity::game;
use log::debug;
use sea_orm::DbConn;
use tokio::task::JoinHandle;

/// One-time idempotent backfill that assigns a content anchor to every game that
/// lacks one. Mirrors [`crate::matching::clone::populate_all_clone_of_ids`]: a
/// game already carrying an anchor is never revisited, so a second run is a
/// no-op. Mapping convergence is left to the reconcile wave on the next cron
/// tick.
pub async fn assign_all_content_anchors(conn: &DbConn) -> anyhow::Result<()> {
	let games = get_games_without_content_anchor(conn).await?;

	for games_chunk in games.chunks(*PARALLELISM) {
		let mut futures: Vec<JoinHandle<anyhow::Result<()>>> = vec![];

		for game in games_chunk.iter() {
			let conn = conn.clone();
			futures.push(tokio::spawn(try_assign_anchor(game.clone(), conn)));
		}

		for future in futures {
			future.await??;
		}
	}

	debug!(
		"Finished backfilling content anchors for {} games",
		games.len()
	);

	Ok(())
}

async fn try_assign_anchor(game: game::Model, conn: DbConn) -> anyhow::Result<()> {
	assign_content_anchor_for_game(&game, &conn).await?;
	Ok(())
}
