use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let conn = manager.get_connection();

		conn.execute_unprepared(
			"ALTER TABLE signature_metadata_mapping \
			 ADD COLUMN IF NOT EXISTS cross_match_last_tried_at TIMESTAMP WITH TIME ZONE NULL;",
		)
		.await?;

		// Partial index supporting the cross-provider name pass query: pulls
		// failed-no-direct-match rows for one provider that have not been
		// cross-matched recently. NULLS FIRST lets fresh rows (never tried)
		// surface ahead of stamped ones.
		conn.execute_unprepared(
			"CREATE INDEX IF NOT EXISTS idx_smm_cross_match_pending \
			 ON signature_metadata_mapping (provider, cross_match_last_tried_at NULLS FIRST, game_id) \
			 WHERE match_type = 'failed'::match_type_enum \
			   AND failed_match_reason = 'no_direct_match'::failed_match_reason_enum;",
		)
		.await?;

		// Partial index supporting the EXISTS subquery on the sibling side:
		// looking up mappings for the same game that another provider matched
		// AND populated `matched_name`. Narrow to the rows that can actually
		// contribute, so the index stays small.
		conn.execute_unprepared(
			"CREATE INDEX IF NOT EXISTS idx_smm_sibling_matched_name \
			 ON signature_metadata_mapping (game_id, provider) \
			 WHERE matched_name IS NOT NULL \
			   AND match_type IN ('automatic'::match_type_enum, 'manual'::match_type_enum);",
		)
		.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let conn = manager.get_connection();
		conn.execute_unprepared("DROP INDEX IF EXISTS idx_smm_sibling_matched_name;")
			.await?;
		conn.execute_unprepared("DROP INDEX IF EXISTS idx_smm_cross_match_pending;")
			.await?;
		conn.execute_unprepared(
			"ALTER TABLE signature_metadata_mapping DROP COLUMN IF EXISTS cross_match_last_tried_at;",
		)
		.await?;
		Ok(())
	}
}
