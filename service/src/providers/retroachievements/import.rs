use crate::db::retroachievements::{
	bulk_insert_retroachievements_hashes, bulk_upsert_retroachievements_games,
	bulk_upsert_retroachievements_systems, record_retroachievements_import,
	truncate_retroachievements_games_and_hashes,
};
use crate::matching::util::normalize_title;
use crate::providers::retroachievements::RetroAchievementsClient;
use crate::providers::retroachievements::api;
use entity::{retroachievements_game, retroachievements_game_hash, retroachievements_system};
use log::{debug, info};
use sea_orm::ActiveValue::Set;
use std::time::{Duration, Instant};
use tokio::time::sleep;

const RA_BATCH_SIZE: usize = 1000;
/// Courtesy delay between per-system `GetGameList` calls. RA does not
/// document a hard rate limit but asks consumers to be considerate.
const RA_PER_REQUEST_DELAY_MS: u64 = 1000;

#[derive(Debug, Default, Clone)]
pub struct ImportOutcome {
	pub systems: usize,
	pub games: usize,
	pub hashes: usize,
	pub elapsed_ms: u128,
}

/// Pulls every RA system, then for each system pulls the game + hash list and
/// bulk-replaces the local tables. No content-hash skip because RA's data
/// changes daily; the import cost is the price of currency.
pub async fn ensure_imported(client: &RetroAchievementsClient) -> anyhow::Result<ImportOutcome> {
	let started = Instant::now();
	info!("RetroAchievements metadata import starting");

	let db_conn = client.db_conn();

	let consoles = api::get_console_ids(client).await?;
	let mut system_rows: Vec<retroachievements_system::ActiveModel> =
		Vec::with_capacity(consoles.len());
	for c in &consoles {
		system_rows.push(retroachievements_system::ActiveModel {
			system_id: Set(c.id),
			name: Set(c.name.clone()),
			..Default::default()
		});
	}
	let system_count = system_rows.len();
	bulk_upsert_retroachievements_systems(system_rows, db_conn).await?;
	crate::metrics::record_retroachievements_import_records(
		"system",
		"imported",
		system_count as u64,
	);
	info!("RetroAchievements: refreshed {system_count} systems, fetching games per system");

	truncate_retroachievements_games_and_hashes(db_conn).await?;

	let mut counts = ImportOutcome {
		systems: system_count,
		..Default::default()
	};

	let total_systems = consoles.len();
	for (idx, console) in consoles.iter().enumerate() {
		// First console always runs without delay; subsequent ones get the
		// courtesy interval.
		if idx > 0 {
			sleep(Duration::from_millis(RA_PER_REQUEST_DELAY_MS)).await;
		}

		let games = match api::get_game_list(client, console.id).await {
			Ok(games) => games,
			Err(e) => {
				log::warn!(
					"RetroAchievements: skipping system {} ({}) due to error: {e}",
					console.id,
					console.name
				);
				crate::metrics::record_retroachievements_import_records(
					"system",
					"skipped_error",
					1,
				);
				continue;
			}
		};

		if games.is_empty() {
			crate::metrics::record_retroachievements_import_records("system", "skipped_empty", 1);
			continue;
		}

		let mut game_batch: Vec<retroachievements_game::ActiveModel> =
			Vec::with_capacity(games.len().min(RA_BATCH_SIZE));
		let mut hash_batch: Vec<retroachievements_game_hash::ActiveModel> =
			Vec::with_capacity(RA_BATCH_SIZE);
		let mut system_game_count = 0usize;
		let mut system_hash_count = 0usize;

		for g in games {
			let title_normalized = Some(normalize_title(&g.title.to_lowercase()));
			game_batch.push(retroachievements_game::ActiveModel {
				game_id: Set(g.id),
				system_id: Set(g.console_id),
				system_name: Set(g.console_name.clone()),
				title: Set(g.title.clone()),
				title_normalized: Set(title_normalized),
				image_icon: Set(g.image_icon.clone()),
				num_achievements: Set(g.num_achievements),
				num_leaderboards: Set(g.num_leaderboards),
				points: Set(g.points),
				date_modified: Set(g.date_modified.clone()),
				forum_topic_id: Set(g.forum_topic_id),
				..Default::default()
			});
			system_game_count += 1;
			for raw_md5 in g.hashes {
				let md5 = raw_md5.trim().to_lowercase();
				if md5.is_empty() {
					continue;
				}
				hash_batch.push(retroachievements_game_hash::ActiveModel {
					game_id: Set(g.id),
					md5: Set(md5),
					..Default::default()
				});
				system_hash_count += 1;
			}
			if game_batch.len() >= RA_BATCH_SIZE {
				let n = game_batch.len();
				bulk_upsert_retroachievements_games(std::mem::take(&mut game_batch), db_conn)
					.await?;
				crate::metrics::record_retroachievements_import_records(
					"game", "imported", n as u64,
				);
			}
			if hash_batch.len() >= RA_BATCH_SIZE {
				let n = hash_batch.len();
				bulk_insert_retroachievements_hashes(std::mem::take(&mut hash_batch), db_conn)
					.await?;
				crate::metrics::record_retroachievements_import_records(
					"hash", "imported", n as u64,
				);
			}
		}
		if !game_batch.is_empty() {
			let n = game_batch.len();
			bulk_upsert_retroachievements_games(game_batch, db_conn).await?;
			crate::metrics::record_retroachievements_import_records("game", "imported", n as u64);
		}
		if !hash_batch.is_empty() {
			let n = hash_batch.len();
			bulk_insert_retroachievements_hashes(hash_batch, db_conn).await?;
			crate::metrics::record_retroachievements_import_records("hash", "imported", n as u64);
		}

		counts.games += system_game_count;
		counts.hashes += system_hash_count;

		if (idx + 1).is_multiple_of(10) {
			info!(
				"RetroAchievements: processed {}/{total_systems} systems ({} games, {} hashes so far)",
				idx + 1,
				counts.games,
				counts.hashes
			);
		}

		debug!(
			"RetroAchievements: system {} ({}) - {system_game_count} games, {system_hash_count} hashes",
			console.id, console.name
		);
	}

	record_retroachievements_import(
		counts.systems as i32,
		counts.games as i32,
		counts.hashes as i32,
		db_conn,
	)
	.await?;

	counts.elapsed_ms = started.elapsed().as_millis();
	info!(
		"RetroAchievements import done in {} ms: {} systems, {} games, {} hashes",
		counts.elapsed_ms, counts.systems, counts.games, counts.hashes,
	);

	Ok(counts)
}
