use sea_orm_migration::prelude::*;

/// Backs the v2 `(name, id)` keyset over signature_group. The other v2 reference
/// lists seek over columns that already carry a unique index (company.name,
/// platform.name); signature_group.name has none, so add a plain btree.
#[derive(Iden)]
enum SignatureGroup {
	Table,
	Name,
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		// Plain CREATE INDEX, not CONCURRENTLY: the migration framework runs each
		// step in a transaction and CONCURRENTLY cannot run there. On a large
		// signature_group table this takes a brief write lock, so deploy it off-peak.
		manager
			.create_index(
				Index::create()
					.if_not_exists()
					.name("signature_group_name_idx")
					.table(SignatureGroup::Table)
					.col(SignatureGroup::Name)
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.drop_index(
				Index::drop()
					.name("signature_group_name_idx")
					.table(SignatureGroup::Table)
					.to_owned(),
			)
			.await
	}
}
