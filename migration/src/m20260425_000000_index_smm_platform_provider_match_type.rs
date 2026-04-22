use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let conn = manager.get_connection();

		// Composite partial index backing the platform-side filter in the
		// unmatched-games query: `smm WHERE provider = :p AND match_type IN
		// ('automatic','manual') AND platform_id IS NOT NULL`. Without it,
		// the 4-way join fans out to every game on every matched platform
		// before the `ORDER BY game.id LIMIT 100` can cut the set, forcing
		// a full sort.
		conn.execute_unprepared(
			r#"
                CREATE INDEX IF NOT EXISTS idx_smm_platform_id_provider_match_type
                  ON signature_metadata_mapping (platform_id, provider, match_type)
                  WHERE platform_id IS NOT NULL;
                "#,
		)
		.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.get_connection()
			.execute_unprepared(r#"DROP INDEX IF EXISTS idx_smm_platform_id_provider_match_type;"#)
			.await?;

		Ok(())
	}
}
