use crate::extension::postgres::Type;
use crate::sea_orm::{EntityTrait, EnumIter, Iterable, Set};
use entity::sea_orm_active_enums::MatchTypeEnum::Manual;
use entity::user;
use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::enumeration;
use sea_orm_migration::sea_orm::ColumnTrait;
use sea_orm_migration::sea_orm::QueryFilter;

#[derive(DeriveIden)]
struct UserPermissionsEnum;

#[derive(DeriveIden, EnumIter)]
pub enum UserPermissions {
	User,
	Trusted,
	Automation,
	Admin,
}

#[derive(Iden)]
enum User {
	Table,
	Id,
	DiscordId,
	Username,
	Permissions,
	ApiKey,
	CreatedAt,
	UpdatedAt,
}

#[derive(Iden)]
enum SignatureMetadataMapping {
	Table,
	ManuallyMatchedBy,
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
	CreatedBy,
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
		manager
			.create_type(
				Type::create()
					.as_enum(UserPermissionsEnum)
					.values(UserPermissions::iter())
					.to_owned(),
			)
			.await?;

		// 1. Create `user` table
		manager
			.create_table(
				Table::create()
					.table(User::Table)
					.if_not_exists()
					.col(
						ColumnDef::new(User::Id)
							.primary_key()
							.uuid()
							.not_null()
							.default(Expr::cust("gen_random_uuid()")),
					)
					.col(ColumnDef::new(User::DiscordId).big_integer().null())
					.col(ColumnDef::new(User::Username).string().not_null())
					.col(ColumnDef::new(User::ApiKey).string().null())
					.col(enumeration(
						User::Permissions,
						UserPermissionsEnum,
						UserPermissions::iter(),
					))
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
					.to_owned(),
			)
			.await?;

		// Create indexes for user table
		manager
			.create_index(
				Index::create()
					.name("idx_user_discord_id")
					.table(User::Table)
					.col(User::DiscordId)
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.name("idx_user_username")
					.table(User::Table)
					.col(User::Username)
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.name("idx_user_api_key")
					.table(User::Table)
					.col(User::ApiKey)
					.to_owned(),
			)
			.await?;

		// 2. Alter `signature_metadata_mapping` → add `created_by` UUID FK + index
		manager
			.alter_table(
				Table::alter()
					.table(SignatureMetadataMapping::Table)
					.add_column(
						ColumnDef::new(SignatureMetadataMapping::ManuallyMatchedBy)
							.uuid()
							.null(),
					)
					.add_foreign_key(
						&TableForeignKey::new()
							.name("fk_smm_created_by")
							.from_tbl(SignatureMetadataMapping::Table)
							.from_col(SignatureMetadataMapping::ManuallyMatchedBy)
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
					.col(SignatureMetadataMapping::ManuallyMatchedBy)
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
						ColumnDef::new(SignatureMetadataMappingSuggestions::CreatedBy)
							.uuid()
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
					.foreign_key(
						&mut ForeignKey::create()
							.name("fk_smm_sugg_created_by")
							.from(
								SignatureMetadataMappingSuggestions::Table,
								SignatureMetadataMappingSuggestions::CreatedBy,
							)
							.to(User::Table, User::Id)
							.on_delete(ForeignKeyAction::NoAction)
							.on_update(ForeignKeyAction::Cascade)
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

		manager
			.create_index(
				Index::create()
					.name("idx_smm_sugg_created_by")
					.table(SignatureMetadataMappingSuggestions::Table)
					.col(SignatureMetadataMappingSuggestions::CreatedBy)
					.to_owned(),
			)
			.await?;

		let admin_user_id = "52901957-53f2-4bb4-82a6-ff86da88490b";

		let initial_admin_user = user::ActiveModel {
			id: Set(admin_user_id.try_into().unwrap()),
			discord_id: Set(Some(184632227894657025)),
			username: Set("DevYukine".to_string()),
			permissions: Set(entity::sea_orm_active_enums::UserPermissionsEnum::Admin),
			..Default::default()
		};

		let initial_bot_user = user::ActiveModel {
			id: Set("b1c8f0d2-3c4e-4f5a-8b6c-7d8e9f0a1b2c".try_into().unwrap()),
			discord_id: Set(Some(1281668958935584779)),
			username: Set("RetroBot".to_string()),
			permissions: Set(entity::sea_orm_active_enums::UserPermissionsEnum::Automation),
			..Default::default()
		};

		let db = manager.get_connection();

		// 5. Insert initial admin and bot users
		user::Entity::insert_many(vec![initial_admin_user, initial_bot_user])
			.exec(db)
			.await?;

		entity::signature_metadata_mapping::Entity::update_many()
			.filter(
				entity::signature_metadata_mapping::Column::MatchType
					.eq(Manual)
					.and(entity::signature_metadata_mapping::Column::ManuallyMatchedBy.is_null()),
			)
			.set(entity::signature_metadata_mapping::ActiveModel {
				manually_matched_by: Set(Some(admin_user_id.try_into().unwrap())),
				..Default::default()
			})
			.exec(db)
			.await?;

		Ok(())
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		// 1. Drop indexes for users and suggestions
		manager
			.drop_index(
				Index::drop()
					.if_exists()
					.name("idx_user_discord_id")
					.table(User::Table)
					.to_owned(),
			)
			.await?;
		manager
			.drop_index(
				Index::drop()
					.if_exists()
					.name("idx_user_username")
					.table(User::Table)
					.to_owned(),
			)
			.await?;
		manager
			.drop_index(
				Index::drop()
					.if_exists()
					.name("idx_user_api_key")
					.table(User::Table)
					.to_owned(),
			)
			.await?;
		manager
			.drop_index(
				Index::drop()
					.if_exists()
					.name("idx_smm_sugg_platform_id")
					.table(SignatureMetadataMappingSuggestions::Table)
					.to_owned(),
			)
			.await?;
		manager
			.drop_index(
				Index::drop()
					.if_exists()
					.name("idx_smm_sugg_company_id")
					.table(SignatureMetadataMappingSuggestions::Table)
					.to_owned(),
			)
			.await?;
		manager
			.drop_index(
				Index::drop()
					.if_exists()
					.name("idx_smm_sugg_game_id")
					.table(SignatureMetadataMappingSuggestions::Table)
					.to_owned(),
			)
			.await?;
		manager
			.drop_index(
				Index::drop()
					.if_exists()
					.name("idx_smm_sugg_created_by")
					.table(SignatureMetadataMappingSuggestions::Table)
					.to_owned(),
			)
			.await?;

		// 2. Drop suggestions table
		manager
			.drop_table(
				Table::drop()
					.if_exists()
					.table(SignatureMetadataMappingSuggestions::Table)
					.to_owned(),
			)
			.await?;

		// 3. Drop index on created_by
		manager
			.drop_index(
				Index::drop()
					.if_exists()
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
					.drop_column(SignatureMetadataMapping::ManuallyMatchedBy)
					.to_owned(),
			)
			.await?;

		// 5. Drop user table
		manager
			.drop_table(Table::drop().if_exists().table(User::Table).to_owned())
			.await?;

		// 6. Drop user permissions enum
		manager
			.drop_type(Type::drop().name(UserPermissionsEnum).to_owned())
			.await?;

		Ok(())
	}
}
