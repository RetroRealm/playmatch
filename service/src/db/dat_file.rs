use crate::db::abstraction::ColumnNullTrait;
use crate::db::pagination::{KeysetPage, fetch_keyset_page};
use entity::prelude::DatFile;
use entity::{dat_file, dat_file_import, game};
use sea_orm::ActiveValue::Set;
use sea_orm::prelude::Uuid;
use sea_orm::sea_query::Expr;
use sea_orm::sea_query::extension::postgres::{PgBinOper, PgExpr};
use sea_orm::{
	ActiveModelTrait, ColumnTrait, DbConn, DbErr, EntityTrait, IntoActiveModel, JoinType,
	PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, RelationTrait, TryIntoModel,
};
use std::collections::HashMap;

/// Parameters for [`create_or_update_dat_file`].
pub struct DatFileCreateOrUpdateInput {
	pub signature_group_id: Uuid,
	pub sanitized_file_name: String,
	pub current_version: String,
	pub tags: Vec<String>,
	pub subset: Option<String>,
	pub company_id: Option<Uuid>,
	pub platform_id: Uuid,
}

/// Filters narrowing the v2 dat-file catalogue browse. All are AND-combined;
/// `None`/empty means the filter is not applied. `tag` matches membership in the
/// dat file's `tags` array; `name_contains` is a case-insensitive substring.
#[derive(Debug, Clone, Default)]
pub struct DatFileBrowseFilters {
	pub signature_group_id: Option<Uuid>,
	pub platform_id: Option<Uuid>,
	pub company_id: Option<Uuid>,
	pub subset: Option<String>,
	pub tag: Option<String>,
	pub name_contains: Option<String>,
}

fn dat_files_browse_base(filters: &DatFileBrowseFilters) -> sea_orm::Select<DatFile> {
	let mut select = DatFile::find();

	if let Some(signature_group_id) = filters.signature_group_id {
		select = select.filter(dat_file::Column::SignatureGroupId.eq(signature_group_id));
	}
	if let Some(platform_id) = filters.platform_id {
		select = select.filter(dat_file::Column::PlatformId.eq(platform_id));
	}
	if let Some(company_id) = filters.company_id {
		select = select.filter(dat_file::Column::CompanyId.eq(company_id));
	}
	if let Some(subset) = filters.subset.as_deref().filter(|s| !s.is_empty()) {
		select = select.filter(dat_file::Column::Subset.eq(subset));
	}
	if let Some(tag) = filters.tag.as_deref().filter(|s| !s.is_empty()) {
		// tags is a Postgres text[]; @> tests array membership and avoids a substring
		// false positive. The array is passed as a bound value so sea_query numbers
		// the placeholder itself; a hardcoded $1 in cust_with_values collides with the
		// keyset cursor's own parameters and silently matches nothing.
		select = select.filter(
			Expr::col((dat_file::Entity, dat_file::Column::Tags))
				.binary(PgBinOper::Contains, Expr::val(vec![tag.to_owned()])),
		);
	}
	if let Some(name) = filters.name_contains.as_deref().filter(|s| !s.is_empty()) {
		select = select.filter(
			Expr::col((dat_file::Entity, dat_file::Column::Name)).ilike(format!("%{name}%")),
		);
	}

	select
}

/// Fetch one keyset page of dat files ordered by `(name, id)`, narrowed by
/// `filters`. Seeks past `after` when supplied. Replaces the former unfiltered
/// `.all()` reader; the N+1 overflow row drives `has_more`.
pub async fn find_all_dat_files(
	filters: &DatFileBrowseFilters,
	after: Option<(String, Uuid)>,
	limit: Option<u64>,
	conn: &DbConn,
) -> Result<KeysetPage<dat_file::Model>, DbErr> {
	let mut cursor =
		dat_files_browse_base(filters).cursor_by((dat_file::Column::Name, dat_file::Column::Id));
	if let Some((name, id)) = after {
		cursor.after((name, id));
	}
	fetch_keyset_page(&mut cursor, limit, conn).await
}

/// Return the id of every dat file. Used by the clone-resolution maintenance
/// pass, which fans out over each dat file and needs only the identifiers.
pub async fn find_all_dat_file_ids(conn: &DbConn) -> Result<Vec<Uuid>, DbErr> {
	DatFile::find()
		.select_only()
		.column(dat_file::Column::Id)
		.into_tuple::<Uuid>()
		.all(conn)
		.await
}

pub async fn get_dat_file_by_id(id: Uuid, conn: &DbConn) -> Result<Option<dat_file::Model>, DbErr> {
	DatFile::find_by_id(id).one(conn).await
}

pub async fn get_latest_import_for_dat_file(
	dat_file_id: Uuid,
	conn: &DbConn,
) -> Result<Option<dat_file_import::Model>, DbErr> {
	dat_file_import::Entity::find()
		.filter(dat_file_import::Column::DatFileId.eq(dat_file_id))
		.order_by_desc(dat_file_import::Column::ImportedAt)
		.order_by_desc(dat_file_import::Column::Id)
		.one(conn)
		.await
}

/// Bulk variant of [`get_latest_import_for_dat_file`]. Loads the imports for
/// every dat file in `dat_file_ids` in one query ordered by `(imported_at, id)`
/// descending, then keeps the first (newest) per dat file. The per-dat-file
/// winner is identical to the single-row reader's `ORDER BY ... LIMIT 1`.
pub async fn get_latest_imports_for_dat_files(
	dat_file_ids: &[Uuid],
	conn: &DbConn,
) -> Result<HashMap<Uuid, dat_file_import::Model>, DbErr> {
	if dat_file_ids.is_empty() {
		return Ok(HashMap::new());
	}

	let imports = dat_file_import::Entity::find()
		.filter(dat_file_import::Column::DatFileId.is_in(dat_file_ids.iter().copied()))
		.order_by_desc(dat_file_import::Column::ImportedAt)
		.order_by_desc(dat_file_import::Column::Id)
		.all(conn)
		.await?;

	let mut latest: HashMap<Uuid, dat_file_import::Model> = HashMap::new();
	for import in imports {
		latest.entry(import.dat_file_id).or_insert(import);
	}
	Ok(latest)
}

pub async fn count_games_in_dat_file(dat_file_id: Uuid, conn: &DbConn) -> Result<u64, DbErr> {
	game::Entity::find()
		.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
		.filter(dat_file_import::Column::DatFileId.eq(dat_file_id))
		.count(conn)
		.await
}

pub async fn count_current_games_in_dat_file(
	dat_file_id: Uuid,
	conn: &DbConn,
) -> Result<u64, DbErr> {
	game::Entity::find()
		.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
		.filter(dat_file_import::Column::DatFileId.eq(dat_file_id))
		.filter(game::Column::IsCurrent.eq(true))
		.count(conn)
		.await
}

fn games_in_dat_file_base(dat_file_id: Uuid, current_only: bool) -> sea_orm::Select<game::Entity> {
	let mut select = game::Entity::find()
		.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
		.filter(dat_file_import::Column::DatFileId.eq(dat_file_id));
	if current_only {
		select = select.filter(game::Column::IsCurrent.eq(true));
	}
	select
}

pub async fn find_games_in_dat_file_page(
	dat_file_id: Uuid,
	current_only: bool,
	after: Option<(String, Uuid)>,
	limit: Option<u64>,
	conn: &DbConn,
) -> Result<KeysetPage<game::Model>, DbErr> {
	let mut cursor = games_in_dat_file_base(dat_file_id, current_only)
		.cursor_by((game::Column::Name, game::Column::Id));
	if let Some((name, id)) = after {
		cursor.after((name, id));
	}
	fetch_keyset_page(&mut cursor, limit, conn).await
}

/// Look up a dat file by signature group, name, company and platform.
/// If it exists and the current version differs, the version is updated in place.
/// If it does not exist, a new row is inserted.
pub async fn create_or_update_dat_file(
	input: DatFileCreateOrUpdateInput,
	conn: &DbConn,
) -> anyhow::Result<dat_file::Model> {
	let dat_file = DatFile::find()
		.filter(dat_file::Column::SignatureGroupId.eq(input.signature_group_id))
		.filter(dat_file::Column::Name.eq(input.sanitized_file_name.clone()))
		.filter(dat_file::Column::CompanyId.eq_null(input.company_id))
		.filter(dat_file::Column::PlatformId.eq(input.platform_id))
		.one(conn)
		.await?;

	if let Some(dat_file) = dat_file {
		if dat_file.current_version != input.current_version {
			let mut active_model = dat_file.into_active_model();
			active_model.current_version = Set(input.current_version.to_string());

			return Ok(active_model.save(conn).await?.try_into_model()?);
		}

		return Ok(dat_file);
	}

	let dat_file = dat_file::ActiveModel {
		signature_group_id: Set(input.signature_group_id),
		name: Set(input.sanitized_file_name.clone()),
		current_version: Set(input.current_version.clone()),
		company_id: Set(input.company_id),
		platform_id: Set(input.platform_id),
		tags: Set(if input.tags.is_empty() {
			None
		} else {
			Some(input.tags)
		}),
		subset: Set(input.subset),
		..Default::default()
	};

	Ok(dat_file.save(conn).await?.try_into_model()?)
}
