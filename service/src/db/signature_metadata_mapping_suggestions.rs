use crate::db::abstraction::ColumnNullTrait;
use crate::db::pagination::{KeysetPage, fetch_keyset_page};
use chrono::{DateTime, Utc};
use entity::sea_orm_active_enums::MetadataProviderEnum;
use entity::signature_metadata_mapping_suggestions::{ActiveModel, Column, Entity, Model};
use sea_orm::prelude::Uuid;
use sea_orm::{ColumnTrait, DbConn, DbErr, EntityTrait, PaginatorTrait, QueryFilter};

pub async fn get_all_suggestions(db_conn: &DbConn) -> Result<Vec<Model>, DbErr> {
	let suggestions = Entity::find().all(db_conn).await?;

	Ok(suggestions)
}

/// Fetch one keyset page of suggestions newest first, ordered by
/// `(created_at, id)` descending. `after` is the last row of the previous page;
/// the next page holds rows strictly older than it. The N+1 overflow row drives
/// `has_more`.
pub async fn find_suggestions_page(
	after: Option<(DateTime<Utc>, Uuid)>,
	limit: Option<u64>,
	db_conn: &DbConn,
) -> Result<KeysetPage<Model>, DbErr> {
	let mut cursor = Entity::find().cursor_by((Column::CreatedAt, Column::Id));
	cursor.desc();
	if let Some((created_at, id)) = after {
		cursor.after((created_at, id));
	}
	fetch_keyset_page(&mut cursor, limit, db_conn).await
}

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

pub async fn insert_suggestion(suggestion: ActiveModel, db_conn: &DbConn) -> Result<Model, DbErr> {
	let suggestion = Entity::insert(suggestion)
		.exec_with_returning(db_conn)
		.await?;

	Ok(suggestion)
}

pub async fn get_suggestion_by_id(id: Uuid, db_conn: &DbConn) -> Result<Option<Model>, DbErr> {
	let suggestion = Entity::find_by_id(id).one(db_conn).await?;

	Ok(suggestion)
}

pub async fn delete_suggestion_by_id(id: Uuid, db_conn: &DbConn) -> Result<(), DbErr> {
	Entity::delete_by_id(id).exec(db_conn).await?;

	Ok(())
}
