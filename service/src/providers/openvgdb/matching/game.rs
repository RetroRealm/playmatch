use crate::db::game::{
	get_automatic_match_failed_games_with_limit, get_unmatched_games_with_clone_of_with_limit,
	get_unmatched_games_without_clone_of_with_limit,
};
use crate::db::game_file::get_game_files_from_game_id;
use crate::db::openvgdb::{
	find_openvgdb_releases_for_rom, find_openvgdb_rom_by_crc, find_openvgdb_rom_by_md5,
	find_openvgdb_rom_by_sha1,
};
use crate::matching::name_parse::parse_name;
use crate::providers::openvgdb::OpenVgdbClient;
use crate::providers::{
	DEFAULT_CHUNK_SIZE, MetadataProvider, Target, drive_match_pipeline, write_auto_match_failed,
	write_auto_match_success,
};
use entity::game::Model;
use entity::openvgdb_rom;
use entity::sea_orm_active_enums::{
	AutomaticMatchReasonEnum, FailedMatchReasonEnum, MetadataProviderEnum,
};
use futures_util::future::BoxFuture;
use log::debug;
use sea_orm::DbConn;
use std::sync::Arc;

pub async fn match_games_to_openvgdb(
	client: Arc<OpenVgdbClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	drive_match_pipeline(
		"game",
		MetadataProviderEnum::OpenVGDB,
		get_unmatched_games_without_clone_of_with_limit,
		match_game_to_openvgdb,
		client.clone(),
		db_conn,
		DEFAULT_CHUNK_SIZE,
	)
	.await?;
	debug!("Finished matching games without clone_of id to OpenVGDB");

	drive_match_pipeline(
		"game",
		MetadataProviderEnum::OpenVGDB,
		get_unmatched_games_with_clone_of_with_limit,
		match_clone_of_game_to_openvgdb,
		client.clone(),
		db_conn,
		DEFAULT_CHUNK_SIZE,
	)
	.await?;
	debug!("Finished matching games with clone_of id to OpenVGDB");

	drive_match_pipeline(
		"game",
		MetadataProviderEnum::OpenVGDB,
		get_automatic_match_failed_games_with_limit,
		match_game_to_openvgdb,
		client,
		db_conn,
		DEFAULT_CHUNK_SIZE,
	)
	.await?;
	debug!("Finished retrying previously failed OpenVGDB game matches");

	Ok(())
}

fn match_clone_of_game_to_openvgdb(
	game: Model,
	client: Arc<OpenVgdbClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		crate::providers::drive_clone_propagation(game, client, db_conn, match_game_to_openvgdb)
			.await
	})
}

fn match_game_to_openvgdb(
	game: Model,
	client: Arc<OpenVgdbClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		let mut redis_conn = client.redis_conn().clone();

		let parsed_dat = parse_name(&game.name);
		let dat_regions: Vec<&'static str> = parsed_dat
			.regions
			.iter()
			.flat_map(|r| r.ovgdb_codes().iter().copied())
			.collect();

		if let Some(()) =
			try_match_by_hashes(&game, &dat_regions, &db_conn, &mut redis_conn).await?
		{
			return Ok(());
		}

		debug!("No OpenVGDB match found for Game \"{}\"", &game.name);
		write_auto_match_failed(
			"openvgdb",
			MetadataProviderEnum::OpenVGDB,
			Target::Game(game.id),
			FailedMatchReasonEnum::NoDirectMatch,
			&db_conn,
			&mut redis_conn,
		)
		.await?;

		Ok(())
	})
}

/// Walks every game file's sha1/md5/crc against the imported `openvgdb_rom`
/// table, returning `Some(())` once a hash hits a rom that carries at least
/// one release.
async fn try_match_by_hashes(
	game: &Model,
	dat_regions: &[&str],
	db_conn: &DbConn,
	redis_conn: &mut redis::aio::MultiplexedConnection,
) -> anyhow::Result<Option<()>> {
	let files = get_game_files_from_game_id(game.id, db_conn).await?;

	for file in &files {
		if let Some(sha1) = file
			.sha1
			.as_deref()
			.map(str::trim)
			.filter(|s| !s.is_empty())
			&& let Some(rom) = find_openvgdb_rom_by_sha1(sha1, db_conn).await?
			&& let Some(()) = record_hash_match(
				game,
				&rom,
				AutomaticMatchReasonEnum::Sha1Hash,
				dat_regions,
				db_conn,
				redis_conn,
			)
			.await?
		{
			return Ok(Some(()));
		}
		if let Some(md5) = file.md5.as_deref().map(str::trim).filter(|s| !s.is_empty())
			&& let Some(rom) = find_openvgdb_rom_by_md5(md5, db_conn).await?
			&& let Some(()) = record_hash_match(
				game,
				&rom,
				AutomaticMatchReasonEnum::Md5Hash,
				dat_regions,
				db_conn,
				redis_conn,
			)
			.await?
		{
			return Ok(Some(()));
		}
		if let Some(crc) = file.crc.as_deref().map(str::trim).filter(|s| !s.is_empty())
			&& let Some(rom) = find_openvgdb_rom_by_crc(crc, db_conn).await?
			&& let Some(()) = record_hash_match(
				game,
				&rom,
				AutomaticMatchReasonEnum::CrcHash,
				dat_regions,
				db_conn,
				redis_conn,
			)
			.await?
		{
			return Ok(Some(()));
		}
	}

	Ok(None)
}

/// Picks the best release for a matched rom (region priority) and records the
/// mapping. Returns `None` when the rom carries no releases so the caller can
/// keep trying other hashes.
async fn record_hash_match(
	game: &Model,
	rom: &openvgdb_rom::Model,
	reason: AutomaticMatchReasonEnum,
	dat_regions: &[&str],
	db_conn: &DbConn,
	redis_conn: &mut redis::aio::MultiplexedConnection,
) -> anyhow::Result<Option<()>> {
	let releases = find_openvgdb_releases_for_rom(rom.rom_id, dat_regions, db_conn).await?;
	let Some(release) = releases.into_iter().next() else {
		return Ok(None);
	};

	debug!(
		"Matched Game \"{}\" to OpenVGDB Release ID {} ({:?})",
		game.name, release.release_id, reason
	);
	write_auto_match_success(
		"openvgdb",
		MetadataProviderEnum::OpenVGDB,
		Target::Game(game.id),
		release.release_id.to_string(),
		reason,
		Some(release.title_name),
		release.release_year.and_then(|y| i16::try_from(y).ok()),
		db_conn,
		redis_conn,
	)
	.await?;
	Ok(Some(()))
}
