use crate::extension::postgres::{Type, TypeAlterStatement};
use crate::sea_orm::{ColumnTrait, EntityTrait, EnumIter, Iterable, QueryFilter};
use entity::signature_metadata_mapping;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
struct AutomaticMatchReasonEnum;

#[derive(DeriveIden)]
struct AutomaticMatchReason1Enum;

#[derive(DeriveIden, EnumIter)]
pub enum AutomaticMatchReasonOld {
	DirectName,
	AlternativeName,
}

#[derive(DeriveIden, EnumIter)]
pub enum AutomaticMatchReasonNew {
	DirectName,
	AlternativeName,
	ViaParent,
	ViaChild,
	ViaSibling,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.alter_type(
				TypeAlterStatement::new()
					.name(AutomaticMatchReasonEnum)
					.add_value(AutomaticMatchReasonNew::ViaChild),
			)
			.await?;

		manager
			.alter_type(
				TypeAlterStatement::new()
					.name(AutomaticMatchReasonEnum)
					.add_value(AutomaticMatchReasonNew::ViaParent),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		let conn = manager.get_connection();

		// This does some workaround to replace the type with the older one as Postgres does not support dropping enum values
		manager
			.create_type(
				Type::create()
					.as_enum(AutomaticMatchReason1Enum)
					.values(AutomaticMatchReasonOld::iter())
					.to_owned(),
			)
			.await?;

		signature_metadata_mapping::Entity::delete_many()
			.filter(
				signature_metadata_mapping::Column::AutomaticMatchReason.is_not_in(vec![
					entity::sea_orm_active_enums::AutomaticMatchReasonEnum::DirectName,
					entity::sea_orm_active_enums::AutomaticMatchReasonEnum::AlternativeName,
				]),
			)
			.exec(conn)
			.await?;

		let stmt = r#"
			ALTER TABLE signature_metadata_mapping
  				ALTER COLUMN automatic_match_reason TYPE automatic_match_reason1_enum
    				USING (automatic_match_reason::text::automatic_match_reason1_enum);
    	"#;

		conn.execute_unprepared(stmt).await?;

		manager
			.drop_type(Type::drop().name(AutomaticMatchReasonEnum).to_owned())
			.await?;

		// somehow seaorm has some problem with quotes when using their own dsl so we run raw sql here
		let stmt = r#"
			ALTER TYPE automatic_match_reason1_enum RENAME TO automatic_match_reason_enum;
		"#;

		conn.execute_unprepared(stmt).await?;

		Ok(())
	}
}
