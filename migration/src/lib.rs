pub use sea_orm_migration::prelude::*;

mod m20240816_000001_initial_migration;
mod m20240816_172957_insert_signature_group;
mod m20240817_124421_create_updated_at_function_and_triggers;
mod m20240819_001646_add_automatic_match_type_to_signature_metadata_matching;
mod m20240819_151045_rename_signature_metadata_mapping_provider_name_to_provider;
mod m20240819_151911_add_signature_metadata_mapping_check_for_relation_id;
mod m20240819_194749_add_parent_and_sibling_automatic_match_reason;
mod m20240820_154703_add_signature_group_internal_clone_of_id;
mod m20240823_145438_add_signature_metadata_mapping_unique_indexes;
mod m20240827_153244_fix_signature_metadata_mapping_unique_indexes_to_take_provider_into_account;
mod m20250602_180531_add_lowercase_index_for_signature_metadata_mapping;
mod m20250610_162421_make_dat_file_import_md5_hash_consistent;
mod m20250611_134139_add_suggestions_for_manual_metadata_matches;
mod m20250713_163201_add_signature_group_internal_clone_of_id_index;
mod m20250716_174600_add_legacy_signature_group;
mod m20250719_011819_add_case_insensitive_index;
mod m20250725_063924_add_normalized_name_automatic_match_reason;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
	fn migrations() -> Vec<Box<dyn MigrationTrait>> {
		vec![
			Box::new(m20240816_000001_initial_migration::Migration),
			Box::new(m20240816_172957_insert_signature_group::Migration),
			Box::new(m20240817_124421_create_updated_at_function_and_triggers::Migration),
			Box::new(m20240819_001646_add_automatic_match_type_to_signature_metadata_matching::Migration),
			Box::new(m20240819_151045_rename_signature_metadata_mapping_provider_name_to_provider::Migration),
			Box::new(m20240819_151911_add_signature_metadata_mapping_check_for_relation_id::Migration),
			Box::new(m20240819_194749_add_parent_and_sibling_automatic_match_reason::Migration),
			Box::new(m20240820_154703_add_signature_group_internal_clone_of_id::Migration),
			Box::new(m20240823_145438_add_signature_metadata_mapping_unique_indexes::Migration),
			Box::new(m20240827_153244_fix_signature_metadata_mapping_unique_indexes_to_take_provider_into_account::Migration),
			Box::new(m20250602_180531_add_lowercase_index_for_signature_metadata_mapping::Migration),
			Box::new(m20250610_162421_make_dat_file_import_md5_hash_consistent::Migration),
			Box::new(m20250611_134139_add_suggestions_for_manual_metadata_matches::Migration),
			Box::new(m20250713_163201_add_signature_group_internal_clone_of_id_index::Migration),
			Box::new(m20250716_174600_add_legacy_signature_group::Migration),
			Box::new(m20250719_011819_add_case_insensitive_index::Migration),
			Box::new(m20250725_063924_add_normalized_name_automatic_match_reason::Migration),
		]
	}
}
