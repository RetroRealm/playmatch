use crate::db::abstraction::ColumnNullTrait;
use entity::sea_orm_active_enums::MetadataProviderEnum;
use entity::signature_metadata_mapping_suggestions::{ActiveModel, Column, Entity, Model};
use sea_orm::prelude::Uuid;
use sea_orm::{ColumnTrait, DbConn, DbErr, EntityTrait, PaginatorTrait, QueryFilter};

/// Return every metadata match suggestion currently in the database.
pub async fn get_all_suggestions(db_conn: &DbConn) -> Result<Vec<Model>, DbErr> {
	let suggestions = Entity::find().all(db_conn).await?;

	Ok(suggestions)
}

/// Check whether a matching suggestion already exists for the given entity, provider and provider id.
pub async fn suggestion_exists(
	game_id: Option<Uuid>,
	platform_id: Option<Uuid>,
	company_id: Option<Uuid>,
	provider: MetadataProviderEnum,
	provider_id: String,
	db_conn: &DbConn,
) -> Result<bool, DbErr> {
	let count = Entity::find()
		.filter(
			Column::GameId
				.eq_null(game_id)
				.and(Column::PlatformId.eq_null(platform_id))
				.and(Column::CompanyId.eq_null(company_id))
				.and(Column::Provider.eq(provider))
				.and(Column::ProviderId.eq(provider_id)),
		)
		.count(db_conn)
		.await?;

	Ok(count > 0)
}

/// Insert a new metadata match suggestion and return the persisted row.
pub async fn insert_suggestion(suggestion: ActiveModel, db_conn: &DbConn) -> Result<Model, DbErr> {
	let suggestion = Entity::insert(suggestion)
		.exec_with_returning(db_conn)
		.await?;

	Ok(suggestion)
}

/// Load a suggestion by its id.
pub async fn get_suggestion_by_id(id: Uuid, db_conn: &DbConn) -> Result<Option<Model>, DbErr> {
	let suggestion = Entity::find_by_id(id).one(db_conn).await?;

	Ok(suggestion)
}

/// Permanently delete a suggestion row by id.
pub async fn delete_suggestion_by_id(id: Uuid, db_conn: &DbConn) -> Result<(), DbErr> {
	Entity::delete_by_id(id).exec(db_conn).await?;

	Ok(())
}
