use sea_orm::{ConnectionTrait, Database};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

// Partial so the retired indexes cover only the minority of not-current rows.
// idx_game_file_last_seen_import backs the presence insert's WHERE last_seen = $import.
const CREATE_STATEMENTS: [&str; 3] = [
	"CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_game_file_retired \
	 ON game_file (game_id) WHERE is_current = false",
	"CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_game_file_last_seen_import \
	 ON game_file (last_seen_dat_file_import_id) WHERE last_seen_dat_file_import_id IS NOT NULL",
	"CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_game_last_seen_import \
	 ON game (last_seen_dat_file_import_id) WHERE last_seen_dat_file_import_id IS NOT NULL",
];

const DROP_STATEMENTS: [&str; 3] = [
	"DROP INDEX CONCURRENTLY IF EXISTS idx_game_file_retired",
	"DROP INDEX CONCURRENTLY IF EXISTS idx_game_file_last_seen_import",
	"DROP INDEX CONCURRENTLY IF EXISTS idx_game_last_seen_import",
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
		// CREATE INDEX CONCURRENTLY cannot run inside a transaction, and
		// sea-orm-migration wraps every Postgres migration in one. Run the builds
		// on a separate autocommit connection so they never lock writes (no
		// maintenance window needed). Each statement runs on its own; a failed
		// build leaves an INVALID index that must be dropped before retrying.
		run_unmanaged(&CREATE_STATEMENTS).await
	}

	async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
		run_unmanaged(&DROP_STATEMENTS).await
	}
}

async fn run_unmanaged(statements: &[&str]) -> Result<(), DbErr> {
	let url = std::env::var("DATABASE_URL")
		.map_err(|e| DbErr::Custom(format!("DATABASE_URL not set for concurrent index build: {e}")))?;
	let conn = Database::connect(&url).await?;
	for statement in statements {
		conn.execute_unprepared(statement).await?;
	}
	Ok(())
}
