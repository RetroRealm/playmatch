pub use sea_orm_migration::prelude::*;

mod m20240816_000001_initial_migration;
mod m20240816_172957_insert_signature_group;
mod m20240817_124421_create_updated_at_function_and_triggers;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
	fn migrations() -> Vec<Box<dyn MigrationTrait>> {
		vec![
			Box::new(m20240816_000001_initial_migration::Migration),
			Box::new(m20240816_172957_insert_signature_group::Migration),
			Box::new(m20240817_124421_create_updated_at_function_and_triggers::Migration),
		]
	}
}
