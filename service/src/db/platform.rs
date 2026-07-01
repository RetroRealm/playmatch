use crate::db::abstraction::ColumnEqIgnoreCaseTrait;
use crate::db::pagination::{KeysetPage, fetch_keyset_page};
use crate::db::unmatched_entities_with_limit;
use entity::platform::ActiveModel;
use entity::prelude::Platform;
use entity::sea_orm_active_enums::{MatchTypeEnum, MetadataProviderEnum};
use entity::{company, dat_file, dat_file_import, game, platform, signature_metadata_mapping};
use sea_orm::ActiveValue::Set;
use sea_orm::prelude::Uuid;
use sea_orm::sea_query::Expr;
use sea_orm::sea_query::extension::postgres::PgExpr;
use sea_orm::{
	ActiveModelTrait, ColumnTrait, DbConn, DbErr, EntityTrait, JoinType, LoaderTrait, ModelTrait,
	PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, RelationTrait, TryIntoModel,
};
use std::collections::HashMap;

pub async fn get_by_id_and_join_company_and_signature_metadata_mappings(
	id: Uuid,
	conn: &DbConn,
) -> Result<
	Option<(
		platform::Model,
		Option<company::Model>,
		Vec<signature_metadata_mapping::Model>,
	)>,
	DbErr,
> {
	let platform = Platform::find()
		.filter(platform::Column::Id.eq(id))
		.one(conn)
		.await?;

	if let Some(platform) = platform {
		let company = platform.find_related(company::Entity).one(conn).await?;

		let signature_metadata_mappings = platform
			.find_related(signature_metadata_mapping::Entity)
			.all(conn)
			.await?;

		Ok(Some((platform, company, signature_metadata_mappings)))
	} else {
		Ok(None)
	}
}

pub async fn find_all_and_join_company_and_signature_metadata_mappings(
	conn: &DbConn,
) -> Result<
	Vec<(
		platform::Model,
		Option<company::Model>,
		Vec<signature_metadata_mapping::Model>,
	)>,
	DbErr,
> {
	let platforms = Platform::find().all(conn).await?;

	let companies = platforms.load_one(company::Entity, conn).await?;
	let signature_metadata_mappings = platforms
		.load_many(signature_metadata_mapping::Entity, conn)
		.await?;

	let companies = companies
		.into_iter()
		.flatten()
		.collect::<Vec<company::Model>>();

	Ok(platforms
		.into_iter()
		.map(|platform| {
			let company = companies
				.iter()
				.find(|company| {
					if let Some(company_id) = platform.company_id {
						company.id == company_id
					} else {
						false
					}
				})
				.cloned();

			let mappings = signature_metadata_mappings
				.iter()
				.find(|mappings| {
					mappings.iter().any(|mapping| {
						if let Some(platform_id) = mapping.platform_id {
							platform_id == platform.id
						} else {
							false
						}
					})
				})
				.cloned()
				.unwrap_or(Vec::new());
			(platform, company, mappings)
		})
		.collect())
}

pub async fn find_platforms_page_and_join_company_and_signature_metadata_mappings(
	after: Option<(String, Uuid)>,
	limit: Option<u64>,
	conn: &DbConn,
) -> Result<
	KeysetPage<(
		platform::Model,
		Option<company::Model>,
		Vec<signature_metadata_mapping::Model>,
	)>,
	DbErr,
> {
	let mut cursor = Platform::find().cursor_by((platform::Column::Name, platform::Column::Id));
	if let Some((name, id)) = after {
		cursor.after((name, id));
	}
	let page = fetch_keyset_page(&mut cursor, limit, conn).await?;

	let companies = page.rows.load_one(company::Entity, conn).await?;
	let mappings = page
		.rows
		.load_many(signature_metadata_mapping::Entity, conn)
		.await?;

	let rows = page
		.rows
		.into_iter()
		.zip(companies)
		.zip(mappings)
		.map(|((platform, company), mapping)| (platform, company, mapping))
		.collect();

	Ok(KeysetPage {
		rows,
		has_more: page.has_more,
	})
}

/// Fetch one keyset page of platforms whose name matches `query` as a
/// case-insensitive substring (`ILIKE %query%`), ordered by `(name, id)` and
/// joined to their company and signature metadata mappings exactly like
/// [`find_platforms_page_and_join_company_and_signature_metadata_mappings`].
/// Seeks past `after` when supplied; the N+1 overflow row drives `has_more`.
pub async fn search_platforms_by_name_page(
	query: &str,
	after: Option<(String, Uuid)>,
	limit: Option<u64>,
	conn: &DbConn,
) -> Result<
	KeysetPage<(
		platform::Model,
		Option<company::Model>,
		Vec<signature_metadata_mapping::Model>,
	)>,
	DbErr,
> {
	let pattern = format!("%{query}%");

	let mut cursor = Platform::find()
		.filter(Expr::col((platform::Entity, platform::Column::Name)).ilike(pattern))
		.cursor_by((platform::Column::Name, platform::Column::Id));
	if let Some((name, id)) = after {
		cursor.after((name, id));
	}
	let page = fetch_keyset_page(&mut cursor, limit, conn).await?;

	let companies = page.rows.load_one(company::Entity, conn).await?;
	let mappings = page
		.rows
		.load_many(signature_metadata_mapping::Entity, conn)
		.await?;

	let rows = page
		.rows
		.into_iter()
		.zip(companies)
		.zip(mappings)
		.map(|((platform, company), mapping)| (platform, company, mapping))
		.collect();

	Ok(KeysetPage {
		rows,
		has_more: page.has_more,
	})
}

/// Count every platform. Cheap enough on this bounded reference table that the
/// v2 list opts into it when the caller asks for a total.
pub async fn count_platforms(conn: &DbConn) -> Result<u64, DbErr> {
	Platform::find().count(conn).await
}

/// Matches a platform by case-insensitive name, creating one (with optional company) when none exists.
pub async fn create_or_find_platform_by_name(
	name: &str,
	company_id: Option<Uuid>,
	conn: &DbConn,
) -> Result<platform::Model, DbErr> {
	let platform = Platform::find()
		.filter(platform::Column::Name.eq_ignore_case(name))
		.one(conn)
		.await?;

	if let Some(platform) = platform {
		Ok(platform)
	} else {
		let mut platform = ActiveModel {
			name: Set(name.to_string()),
			company_id: Set(company_id),
			..Default::default()
		};

		platform = platform.save(conn).await?;

		Ok(platform.try_into_model()?)
	}
}

unmatched_entities_with_limit! {
	/// Return up to `limit` platforms that have no metadata mapping yet for the given provider
	/// (or one with match_type None). Returns `Ok(None)` when there is nothing left to process.
	get_unmatched_platforms_with_limit,
	Platform,
	platform::Model,
	platform::Column::Id,
	signature_metadata_mapping::Column::PlatformId
}

pub async fn find_platform_of_game(
	game_id: Uuid,
	conn: &DbConn,
) -> Result<Option<platform::Model>, DbErr> {
	Platform::find()
		.join(JoinType::InnerJoin, platform::Relation::DatFile.def())
		.join(JoinType::InnerJoin, dat_file::Relation::DatFileImport.def())
		.join(JoinType::InnerJoin, dat_file_import::Relation::Game.def())
		.filter(game::Column::Id.eq(game_id))
		.one(conn)
		.await
}

pub async fn find_platform_related_signature_metadata_mapping(
	model: &platform::Model,
	provider: MetadataProviderEnum,
	conn: &DbConn,
) -> Result<Option<signature_metadata_mapping::Model>, DbErr> {
	model
		.find_related(signature_metadata_mapping::Entity)
		.filter(signature_metadata_mapping::Column::Provider.eq(provider))
		.one(conn)
		.await
}

/// Looks up a platform by exact, case-sensitive name.
pub async fn find_platform_by_name(
	name: &str,
	conn: &DbConn,
) -> Result<Option<platform::Model>, DbErr> {
	Platform::find()
		.filter(platform::Column::Name.eq(name))
		.one(conn)
		.await
}

/// Load every platform in `ids` in one `is_in` query, keyed by id. Bulk variant
/// used to hydrate a page of dat files without a query per row.
pub async fn find_platforms_by_ids(
	ids: &[Uuid],
	conn: &DbConn,
) -> Result<HashMap<Uuid, platform::Model>, DbErr> {
	if ids.is_empty() {
		return Ok(HashMap::new());
	}
	let rows = Platform::find()
		.filter(platform::Column::Id.is_in(ids.iter().copied()))
		.all(conn)
		.await?;
	Ok(rows.into_iter().map(|row| (row.id, row)).collect())
}
