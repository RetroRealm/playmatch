use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let conn = manager.get_connection();

		// Create the update timestamp function
		let create_function_sql = r#"
        CREATE OR REPLACE FUNCTION update_modified_column()
        RETURNS TRIGGER AS $$
        BEGIN
            NEW.updated_at = CURRENT_TIMESTAMP;
            RETURN NEW;
        END;
        $$ LANGUAGE plpgsql;
        "#;
		conn.execute_unprepared(create_function_sql).await?;

		// Create the trigger for the signature_group table
		let create_signature_group_trigger_sql = r#"
        CREATE TRIGGER update_signature_group_modified_time
        BEFORE UPDATE ON signature_group
        FOR EACH ROW
        EXECUTE PROCEDURE update_modified_column();
        "#;
		conn.execute_unprepared(create_signature_group_trigger_sql)
			.await?;

		// Create the trigger for the posts table
		let create_dat_file_trigger_sql = r#"
        CREATE TRIGGER update_dat_file_modified_time
        BEFORE UPDATE ON dat_file
        FOR EACH ROW
        EXECUTE PROCEDURE update_modified_column();
        "#;
		conn.execute_unprepared(create_dat_file_trigger_sql).await?;

		// Create the trigger for the posts table
		let create_dat_file_import_trigger_sql = r#"
        CREATE TRIGGER update_dat_file_import_modified_time
        BEFORE UPDATE ON dat_file_import
        FOR EACH ROW
        EXECUTE PROCEDURE update_modified_column();
        "#;
		conn.execute_unprepared(create_dat_file_import_trigger_sql)
			.await?;

		// Create the trigger for the posts table
		let create_company_trigger_sql = r#"
        CREATE TRIGGER update_company_modified_time
        BEFORE UPDATE ON company
        FOR EACH ROW
        EXECUTE PROCEDURE update_modified_column();
        "#;
		conn.execute_unprepared(create_company_trigger_sql).await?;

		// Create the trigger for the posts table
		let create_platform_trigger_sql = r#"
        CREATE TRIGGER update_platform_modified_time
        BEFORE UPDATE ON platform
        FOR EACH ROW
        EXECUTE PROCEDURE update_modified_column();
        "#;
		conn.execute_unprepared(create_platform_trigger_sql).await?;

		// Create the trigger for the posts table
		let create_game_trigger_sql = r#"
        CREATE TRIGGER update_game_modified_time
        BEFORE UPDATE ON game
        FOR EACH ROW
        EXECUTE PROCEDURE update_modified_column();
        "#;
		conn.execute_unprepared(create_game_trigger_sql).await?;

		// Create the trigger for the posts table
		let create_game_file_trigger_sql = r#"
        CREATE TRIGGER update_game_file_modified_time
        BEFORE UPDATE ON game_file
        FOR EACH ROW
        EXECUTE PROCEDURE update_modified_column();
        "#;
		conn.execute_unprepared(create_game_file_trigger_sql)
			.await?;

		// Create the trigger for the posts table
		let create_signature_metadata_mapping_trigger_sql = r#"
        CREATE TRIGGER update_signature_metadata_mapping_modified_time
        BEFORE UPDATE ON signature_metadata_mapping
        FOR EACH ROW
        EXECUTE PROCEDURE update_modified_column();
        "#;
		conn.execute_unprepared(create_signature_metadata_mapping_trigger_sql)
			.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let conn = manager.get_connection();

		// Drop triggers
		conn.execute_unprepared(
			"DROP TRIGGER IF EXISTS update_signature_group_modified_time ON signature_group;",
		)
		.await?;
		conn.execute_unprepared(
			"DROP TRIGGER IF EXISTS update_dat_file_modified_time ON dat_file;",
		)
		.await?;
		conn.execute_unprepared(
			"DROP TRIGGER IF EXISTS update_dat_file_import_modified_time ON dat_file_import;",
		)
		.await?;
		conn.execute_unprepared("DROP TRIGGER IF EXISTS update_company_modified_time ON company;")
			.await?;
		conn.execute_unprepared(
			"DROP TRIGGER IF EXISTS update_platform_modified_time ON platform;",
		)
		.await?;
		conn.execute_unprepared("DROP TRIGGER IF EXISTS update_game_modified_time ON game;")
			.await?;
		conn.execute_unprepared(
			"DROP TRIGGER IF EXISTS update_game_file_modified_time ON game_file;",
		)
		.await?;
		conn.execute_unprepared("DROP TRIGGER IF EXISTS update_signature_metadata_mapping_modified_time ON signature_metadata_mapping;")
			.await?;

		// Drop the function
		conn.execute_unprepared("DROP FUNCTION IF EXISTS update_modified_column;")
			.await?;

		Ok(())
	}
}
