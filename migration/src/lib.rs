pub use sea_orm_migration::prelude::*;

mod m20240816_000001_initial_migration;
mod m20240816_172957_insert_signature_group;
mod m20240817_124421_create_updated_at_function_and_triggers;
mod m20240819_001646_add_automatic_match_type_to_signature_metadata_matching;
mod m20240819_151045_rename_signature_metadata_mapping_provider_name_to_provider;
mod m20240819_151911_add_signature_metadata_mapping_check_for_relation_id;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
	fn migrations() -> Vec<Box<dyn MigrationTrait>> {
		vec![
			Box::new(m20240816_000001_initial_migration::Migration),
			Box::new(m20240816_172957_insert_signature_group::Migration),
			Box::new(m20240817_124421_create_updated_at_function_and_triggers::Migration),
			Box::new(
				m20240819_001646_add_automatic_match_type_to_signature_metadata_matching::Migration,
			),
			Box::new(
				m20240819_151045_rename_signature_metadata_mapping_provider_name_to_provider::Migration,
			),
			Box::new(
				m20240819_151911_add_signature_metadata_mapping_check_for_relation_id::Migration,
			),
		]
	}
}
