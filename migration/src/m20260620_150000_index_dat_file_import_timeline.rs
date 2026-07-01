use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let conn = manager.get_connection();

		// Backs the v2 newest-first import timeline, which filters by dat_file_id
		// and orders by (imported_at, id) descending. The plain imported_at btree
		// from the initial migration does not cover the dat_file_id seek, so paging
		// a single dat file's history would scan and sort without this.
		//
		// Plain CREATE INDEX, not CONCURRENTLY: the migration framework runs each
		// step in a transaction and CONCURRENTLY cannot run there. On a large
		// dat_file_import table this takes a brief write lock, so deploy it off-peak.
		conn.execute_unprepared(
			r#"
            CREATE INDEX IF NOT EXISTS idx_dat_file_import_dat_file_id_imported_at_id
              ON dat_file_import (dat_file_id, imported_at DESC, id DESC);
            "#,
		)
		.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.get_connection()
			.execute_unprepared(
				r#"DROP INDEX IF EXISTS idx_dat_file_import_dat_file_id_imported_at_id;"#,
			)
			.await?;

		Ok(())
	}
}
