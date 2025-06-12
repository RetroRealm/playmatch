use sea_orm_migration::prelude::*;

#[derive(Iden)]
enum User {
	Table,
	Id,
	Name,
	DiscordId,
	CreatedAt,
	UpdatedAt,
}

#[derive(Iden)]
enum SignatureMetadataMapping {
	Table,
	CreatedBy,
}

#[derive(Iden)]
enum SignatureMetadataMappingSuggestions {
	Table,
	Id,
	GameId,
	CompanyId,
	PlatformId,
	Provider,
	ProviderId,
	Comment,
	CreatedAt,
	UpdatedAt,
}

#[derive(Iden)]
enum Game {
	Table,
	Id,
}

#[derive(Iden)]
enum Company {
	Table,
	Id,
}

#[derive(Iden)]
enum Platform {
	Table,
	Id,
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		// 1. Create `user` table
		manager
			.create_table(
				Table::create()
					.table(User::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(User::Id)
							.uuid()
							.not_null()
							.default(Expr::cust("gen_random_uuid()")),
					)
					.col(ColumnDef::new(User::Name).string().not_null())
					.col(ColumnDef::new(User::DiscordId).string().not_null())
					.col(
						ColumnDef::new(User::CreatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::cust("CURRENT_TIMESTAMP")),
					)
					.col(
						ColumnDef::new(User::UpdatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::cust("CURRENT_TIMESTAMP")),
					)
					.primary_key(Index::create().col(User::Id))
					.to_owned(),
			)
			.await?;

		// 2. Alter `signature_metadata_mapping` → add `created_by` UUID FK + index
		manager
			.alter_table(
				Table::alter()
					.table(SignatureMetadataMapping::Table)
					.add_column(
						ColumnDef::new(SignatureMetadataMapping::CreatedBy)
							.uuid()
							.null(),
					)
					.add_foreign_key(
						&TableForeignKey::new()
							.name("fk_smm_created_by")
							.from_tbl(SignatureMetadataMapping::Table)
							.from_col(SignatureMetadataMapping::CreatedBy)
							.to_tbl(User::Table)
							.to_col(User::Id)
							.on_delete(ForeignKeyAction::SetNull)
							.on_update(ForeignKeyAction::Cascade)
							.to_owned(),
					)
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.name("idx_smm_created_by")
					.table(SignatureMetadataMapping::Table)
					.col(SignatureMetadataMapping::CreatedBy)
					.to_owned(),
			)
			.await?;

		// 3. Create `signature_metadata_mapping_suggestions`
		manager
			.create_table(
				Table::create()
					.table(SignatureMetadataMappingSuggestions::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(SignatureMetadataMappingSuggestions::Id)
							.uuid()
							.not_null()
							.default(Expr::cust("gen_random_uuid()")),
					)
					.col(
						ColumnDef::new(SignatureMetadataMappingSuggestions::GameId)
							.uuid()
							.null(),
					)
					.col(
						ColumnDef::new(SignatureMetadataMappingSuggestions::CompanyId)
							.uuid()
							.null(),
					)
					.col(
						ColumnDef::new(SignatureMetadataMappingSuggestions::PlatformId)
							.uuid()
							.null(),
					)
					.col(
						ColumnDef::new(SignatureMetadataMappingSuggestions::Provider)
							.custom("metadata_provider_enum")
							.not_null(),
					)
					.col(
						ColumnDef::new(SignatureMetadataMappingSuggestions::ProviderId)
							.string()
							.not_null(),
					)
					.col(
						ColumnDef::new(SignatureMetadataMappingSuggestions::Comment)
							.string()
							.null(),
					)
					.col(
						ColumnDef::new(SignatureMetadataMappingSuggestions::CreatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::cust("CURRENT_TIMESTAMP")),
					)
					.col(
						ColumnDef::new(SignatureMetadataMappingSuggestions::UpdatedAt)
							.timestamp_with_time_zone()
							.not_null()
							.default(Expr::cust("CURRENT_TIMESTAMP")),
					)
					.primary_key(Index::create().col(SignatureMetadataMappingSuggestions::Id))
					// FKs to game, company, platform
					.foreign_key(
						&mut ForeignKey::create()
							.name("fk_smm_sugg_game_id")
							.from(
								SignatureMetadataMappingSuggestions::Table,
								SignatureMetadataMappingSuggestions::GameId,
							)
							.to(Game::Table, Game::Id)
							.on_delete(ForeignKeyAction::Cascade)
							.to_owned(),
					)
					.foreign_key(
						&mut ForeignKey::create()
							.name("fk_smm_sugg_company_id")
							.from(
								SignatureMetadataMappingSuggestions::Table,
								SignatureMetadataMappingSuggestions::CompanyId,
							)
							.to(Company::Table, Company::Id)
							.on_delete(ForeignKeyAction::Cascade)
							.to_owned(),
					)
					.foreign_key(
						&mut ForeignKey::create()
							.name("fk_smm_sugg_platform_id")
							.from(
								SignatureMetadataMappingSuggestions::Table,
								SignatureMetadataMappingSuggestions::PlatformId,
							)
							.to(Platform::Table, Platform::Id)
							.on_delete(ForeignKeyAction::Cascade)
							.to_owned(),
					)
					// same “one of game_id, company_id, platform_id” constraint
					.check(Expr::cust(
						"num_nonnulls(game_id, company_id, platform_id) = 1",
					))
					.to_owned(),
			)
			.await?;

		// 4. Indexes for suggestions FK columns
		manager
			.create_index(
				Index::create()
					.name("idx_smm_sugg_game_id")
					.table(SignatureMetadataMappingSuggestions::Table)
					.col(SignatureMetadataMappingSuggestions::GameId)
					.to_owned(),
			)
			.await?;
		manager
			.create_index(
				Index::create()
					.name("idx_smm_sugg_company_id")
					.table(SignatureMetadataMappingSuggestions::Table)
					.col(SignatureMetadataMappingSuggestions::CompanyId)
					.to_owned(),
			)
			.await?;
		manager
			.create_index(
				Index::create()
					.name("idx_smm_sugg_platform_id")
					.table(SignatureMetadataMappingSuggestions::Table)
					.col(SignatureMetadataMappingSuggestions::PlatformId)
					.to_owned(),
			)
			.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		// 1. Drop indexes on suggestions
		manager
			.drop_index(
				Index::drop()
					.name("idx_smm_sugg_platform_id")
					.table(SignatureMetadataMappingSuggestions::Table)
					.to_owned(),
			)
			.await?;
		manager
			.drop_index(
				Index::drop()
					.name("idx_smm_sugg_company_id")
					.table(SignatureMetadataMappingSuggestions::Table)
					.to_owned(),
			)
			.await?;
		manager
			.drop_index(
				Index::drop()
					.name("idx_smm_sugg_game_id")
					.table(SignatureMetadataMappingSuggestions::Table)
					.to_owned(),
			)
			.await?;

		// 2. Drop suggestions table
		manager
			.drop_table(
				Table::drop()
					.table(SignatureMetadataMappingSuggestions::Table)
					.to_owned(),
			)
			.await?;

		// 3. Drop index on created_by
		manager
			.drop_index(
				Index::drop()
					.name("idx_smm_created_by")
					.table(SignatureMetadataMapping::Table)
					.to_owned(),
			)
			.await?;

		// 4. Remove FK & column from signature_metadata_mapping
		manager
			.alter_table(
				Table::alter()
					.table(SignatureMetadataMapping::Table)
					.drop_foreign_key("fk_smm_created_by")
					.drop_column(SignatureMetadataMapping::CreatedBy)
					.to_owned(),
			)
			.await?;

		// 5. Drop user table
		manager
			.drop_table(Table::drop().table(User::Table).to_owned())
			.await?;

		Ok(())
	}
}
