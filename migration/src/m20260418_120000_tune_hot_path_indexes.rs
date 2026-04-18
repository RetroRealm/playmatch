use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let conn = manager.get_connection();

		// Compound index covering the identify fallback (name + size lookup).
		// Subsumes the single-column idx_game_file_lower_file_name dropped below.
		conn.execute_unprepared(
			r#"
                CREATE INDEX IF NOT EXISTS idx_game_file_lower_file_name_size
                  ON game_file ((LOWER("file_name")), file_size_in_bytes);
                "#,
		)
		.await?;

		// Compound index for suggestion_exists dedup lookup.
		conn.execute_unprepared(
			r#"
                CREATE INDEX IF NOT EXISTS idx_smm_suggestions_provider_provider_id
                  ON signature_metadata_mapping_suggestions (provider, provider_id);
                "#,
		)
		.await?;

		// Partial index for the 60-day stale-failure retry cron. Narrow on match_type
		// and failed_match_reason, leaves updated_at for the range scan.
		conn.execute_unprepared(
			r#"
                CREATE INDEX IF NOT EXISTS idx_smm_failed_retry
                  ON signature_metadata_mapping (updated_at)
                  WHERE match_type = 'failed'::match_type_enum
                    AND failed_match_reason = 'no_direct_match'::failed_match_reason_enum;
                "#,
		)
		.await?;

		// Drop plain case-sensitive hash indexes on game_file. All query paths use
		// eq_ignore_case which emits LOWER() SQL and hits the functional indexes only.
		conn.execute_unprepared(r#"DROP INDEX IF EXISTS idx_game_file_md5;"#)
			.await?;
		conn.execute_unprepared(r#"DROP INDEX IF EXISTS idx_game_file_sha1;"#)
			.await?;
		conn.execute_unprepared(r#"DROP INDEX IF EXISTS idx_game_file_sha256;"#)
			.await?;
		conn.execute_unprepared(r#"DROP INDEX IF EXISTS idx_game_file_crc;"#)
			.await?;

		// Drop the single-column lower(file_name) index; superseded by the compound above.
		conn.execute_unprepared(r#"DROP INDEX IF EXISTS idx_game_file_lower_file_name;"#)
			.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let conn = manager.get_connection();

		// Recreate the indexes this migration dropped.
		conn.execute_unprepared(
			r#"CREATE INDEX IF NOT EXISTS idx_game_file_lower_file_name ON game_file ((LOWER("file_name")));"#,
		)
		.await?;
		conn.execute_unprepared(
			r#"CREATE INDEX IF NOT EXISTS idx_game_file_crc ON game_file (crc);"#,
		)
		.await?;
		conn.execute_unprepared(
			r#"CREATE INDEX IF NOT EXISTS idx_game_file_sha256 ON game_file (sha256);"#,
		)
		.await?;
		conn.execute_unprepared(
			r#"CREATE INDEX IF NOT EXISTS idx_game_file_sha1 ON game_file (sha1);"#,
		)
		.await?;
		conn.execute_unprepared(
			r#"CREATE INDEX IF NOT EXISTS idx_game_file_md5 ON game_file (md5);"#,
		)
		.await?;

		// Drop the indexes this migration added.
		conn.execute_unprepared(r#"DROP INDEX IF EXISTS idx_smm_failed_retry;"#)
			.await?;
		conn.execute_unprepared(
			r#"DROP INDEX IF EXISTS idx_smm_suggestions_provider_provider_id;"#,
		)
		.await?;
		conn.execute_unprepared(r#"DROP INDEX IF EXISTS idx_game_file_lower_file_name_size;"#)
			.await?;

		Ok(())
	}
}
