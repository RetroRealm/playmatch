use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

// One-shot bootstrap for tgdb_game and tgdb_game_alias from a TheGamesDB
// export. The fixture is pre-processed locally from the upstream CSV (HTML
// entities decoded, single-quotes escaped, title_normalized + alt_name_normalized
// pre-computed via the runtime normalize_title rules) and emitted as a sequence
// of `INSERT ... ON CONFLICT DO NOTHING` statements separated by `;\n`.
//
// The TheGamesDB free tier permits only ~1000 API requests per month, so the
// CSV-derived seed avoids burning the monthly quota to bootstrap ~120k rows.
const SEED: &str = include_str!("data/tgdb_bootstrap_seed.sql");

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let conn = manager.get_connection();
		for stmt in SEED.split(";\n").map(str::trim).filter(|s| !s.is_empty()) {
			conn.execute_unprepared(stmt).await?;
		}
		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let conn = manager.get_connection();
		conn.execute_unprepared("DELETE FROM tgdb_game_alias;")
			.await?;
		conn.execute_unprepared("DELETE FROM tgdb_game;").await?;
		Ok(())
	}
}
