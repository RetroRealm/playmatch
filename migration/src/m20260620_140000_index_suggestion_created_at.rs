use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let conn = manager.get_connection();

		// Backs the v2 newest-first suggestion keyset, which orders by
		// (created_at, id) descending. The descending btree matches the scan
		// direction so paging avoids a sort.
		//
		// Plain CREATE INDEX, not CONCURRENTLY: the migration framework runs each
		// step in a transaction and CONCURRENTLY cannot run there. On a large
		// suggestion table this takes a brief write lock, so deploy it off-peak.
		conn.execute_unprepared(
			r#"
            CREATE INDEX IF NOT EXISTS idx_smm_suggestions_created_at_id
              ON signature_metadata_mapping_suggestions (created_at DESC, id DESC);
            "#,
		)
		.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.get_connection()
			.execute_unprepared(r#"DROP INDEX IF EXISTS idx_smm_suggestions_created_at_id;"#)
			.await?;

		Ok(())
	}
}
