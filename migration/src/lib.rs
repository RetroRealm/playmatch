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
mod m20260418_120000_tune_hot_path_indexes;
mod m20260420_041040_hash_api_keys;
mod m20260420_120000_add_source_to_suggestions;
mod m20260423_000000_index_game_dat_file_import_id_name;
mod m20260424_000000_add_api_key_hash_hmac_column;
mod m20260425_000000_index_smm_platform_provider_match_type;
mod m20260426_120000_add_steamgriddb_provider_enum_value;
mod m20260427_120000_add_screenscraper_provider_enum_value;
mod m20260427_120100_add_hash_automatic_match_reason_enum_values;
mod m20260428_120000_add_mobygames_provider_enum_value;
mod m20260428_120100_add_launchbox_provider_enum_value;
mod m20260428_120200_create_launchbox_tables;
mod m20260428_120300_add_emuready_provider_enum_value;
mod m20260428_120400_add_matched_name_to_signature_metadata_mapping;
mod m20260428_120500_add_cross_provider_name_match_reasons;
mod m20260429_120000_add_cross_match_last_tried_at_to_smm;
mod m20260430_120000_add_normalized_name_to_launchbox;
mod m20260510_120000_add_matched_year_and_ambiguous_reason;
mod m20260515_120000_add_openvgdb_provider_enum_value;
mod m20260515_120100_create_openvgdb_tables;
mod m20260516_120000_add_openvgdb_release_normalized_title;

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
			Box::new(m20260418_120000_tune_hot_path_indexes::Migration),
			Box::new(m20260420_041040_hash_api_keys::Migration),
			Box::new(m20260420_120000_add_source_to_suggestions::Migration),
			Box::new(m20260423_000000_index_game_dat_file_import_id_name::Migration),
			Box::new(m20260424_000000_add_api_key_hash_hmac_column::Migration),
			Box::new(m20260425_000000_index_smm_platform_provider_match_type::Migration),
			Box::new(m20260426_120000_add_steamgriddb_provider_enum_value::Migration),
			Box::new(m20260427_120000_add_screenscraper_provider_enum_value::Migration),
			Box::new(m20260427_120100_add_hash_automatic_match_reason_enum_values::Migration),
			Box::new(m20260428_120000_add_mobygames_provider_enum_value::Migration),
			Box::new(m20260428_120100_add_launchbox_provider_enum_value::Migration),
			Box::new(m20260428_120200_create_launchbox_tables::Migration),
			Box::new(m20260428_120300_add_emuready_provider_enum_value::Migration),
			Box::new(m20260428_120400_add_matched_name_to_signature_metadata_mapping::Migration),
			Box::new(m20260428_120500_add_cross_provider_name_match_reasons::Migration),
			Box::new(m20260429_120000_add_cross_match_last_tried_at_to_smm::Migration),
			Box::new(m20260430_120000_add_normalized_name_to_launchbox::Migration),
			Box::new(m20260510_120000_add_matched_year_and_ambiguous_reason::Migration),
			Box::new(m20260515_120000_add_openvgdb_provider_enum_value::Migration),
			Box::new(m20260515_120100_create_openvgdb_tables::Migration),
			Box::new(m20260516_120000_add_openvgdb_release_normalized_title::Migration),
		]
	}
}
