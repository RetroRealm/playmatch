use crate::db::abstraction::ColumnEqIgnoreCaseTrait;
use entity::{
	launchbox_game, launchbox_game_alternate_name, launchbox_game_image, launchbox_import,
	launchbox_platform,
};
use sea_orm::ActiveValue::Set;
use sea_orm::sea_query::{CaseStatement, Expr, Func, OnConflict, SimpleExpr};
use sea_orm::{
	ColumnTrait, DbConn, DbErr, EntityTrait, JoinType, Order, QueryFilter, QueryOrder, QuerySelect,
	RelationTrait,
};
use std::collections::HashSet;

const LB_SEARCH_LIMIT: u64 = 50;
/// Bounds the candidate set returned by the per-rung match helpers. Higher
/// than `LB_SEARCH_LIMIT` because alt-name joins can return the same game
/// once per matching alternate row before the Rust-side dedup runs.
const LB_MATCH_LIMIT: u64 = 50;

pub async fn list_lb_platforms(conn: &DbConn) -> Result<Vec<launchbox_platform::Model>, DbErr> {
	launchbox_platform::Entity::find()
		.order_by_asc(launchbox_platform::Column::Name)
		.all(conn)
		.await
}

pub async fn find_lb_game_by_database_id(
	database_id: i64,
	conn: &DbConn,
) -> Result<Option<launchbox_game::Model>, DbErr> {
	launchbox_game::Entity::find()
		.filter(launchbox_game::Column::DatabaseId.eq(database_id))
		.one(conn)
		.await
}

pub async fn search_lb_games(
	platform_name: Option<&str>,
	query: &str,
	conn: &DbConn,
) -> Result<Vec<launchbox_game::Model>, DbErr> {
	let pattern = format!("%{}%", query.to_lowercase());
	let mut q = launchbox_game::Entity::find()
		.filter(Expr::expr(Func::lower(Expr::col(launchbox_game::Column::Name))).like(pattern));
	if let Some(p) = platform_name {
		q = q.filter(launchbox_game::Column::PlatformName.eq_ignore_case(p));
	}
	q.order_by(launchbox_game::Column::Name, Order::Asc)
		.limit(LB_SEARCH_LIMIT)
		.all(conn)
		.await
}

pub async fn find_lb_games_by_platform_and_name(
	platform_name: &str,
	name: &str,
	conn: &DbConn,
) -> Result<Vec<launchbox_game::Model>, DbErr> {
	launchbox_game::Entity::find()
		.filter(
			launchbox_game::Column::PlatformName
				.eq_ignore_case(platform_name)
				.and(launchbox_game::Column::Name.eq_ignore_case(name)),
		)
		.limit(LB_MATCH_LIMIT)
		.all(conn)
		.await
}

pub async fn find_lb_game_by_platform_and_alternate_name(
	platform_name: &str,
	name: &str,
	conn: &DbConn,
) -> Result<Option<launchbox_game::Model>, DbErr> {
	launchbox_game::Entity::find()
		.join(
			JoinType::InnerJoin,
			launchbox_game::Relation::AlternateName.def(),
		)
		.filter(launchbox_game::Column::PlatformName.eq_ignore_case(platform_name))
		.filter(launchbox_game_alternate_name::Column::Name.eq_ignore_case(name))
		.one(conn)
		.await
}

/// `name_normalized` is populated at import time with `normalize_title`,
/// which Postgres `lower()` cannot reproduce (NFKD, symbol stripping,
/// `&` folding).
pub async fn find_lb_games_by_platform_and_normalized_name(
	platform_name: &str,
	normalized_name: &str,
	conn: &DbConn,
) -> Result<Vec<launchbox_game::Model>, DbErr> {
	launchbox_game::Entity::find()
		.filter(
			launchbox_game::Column::PlatformName
				.eq_ignore_case(platform_name)
				.and(launchbox_game::Column::NameNormalized.eq_ignore_case(normalized_name)),
		)
		.limit(LB_MATCH_LIMIT)
		.all(conn)
		.await
}

pub async fn find_lb_game_by_platform_and_alternate_normalized_name(
	platform_name: &str,
	normalized_name: &str,
	conn: &DbConn,
) -> Result<Option<launchbox_game::Model>, DbErr> {
	launchbox_game::Entity::find()
		.join(
			JoinType::InnerJoin,
			launchbox_game::Relation::AlternateName.def(),
		)
		.filter(launchbox_game::Column::PlatformName.eq_ignore_case(platform_name))
		.filter(
			launchbox_game_alternate_name::Column::NameNormalized.eq_ignore_case(normalized_name),
		)
		.one(conn)
		.await
}

/// Order: rows whose `region` is in `prefer_regions` first, then NULL
/// regions, then everything else. The Rust-side dedup keeps the first
/// occurrence per `database_id`, so each game is represented by its
/// best-region alt row.
pub async fn find_lb_games_by_platform_and_alternate_name_region_priority(
	platform_name: &str,
	name: &str,
	prefer_regions: &[&str],
	conn: &DbConn,
) -> Result<Vec<launchbox_game::Model>, DbErr> {
	let rows = launchbox_game::Entity::find()
		.join(
			JoinType::InnerJoin,
			launchbox_game::Relation::AlternateName.def(),
		)
		.filter(launchbox_game::Column::PlatformName.eq_ignore_case(platform_name))
		.filter(launchbox_game_alternate_name::Column::Name.eq_ignore_case(name))
		.order_by(region_priority_case(prefer_regions), Order::Asc)
		.order_by_asc(launchbox_game_alternate_name::Column::Region)
		.limit(LB_MATCH_LIMIT)
		.all(conn)
		.await?;
	Ok(dedup_by_database_id(rows))
}

pub async fn find_lb_games_by_platform_and_alternate_normalized_name_region_priority(
	platform_name: &str,
	normalized_name: &str,
	prefer_regions: &[&str],
	conn: &DbConn,
) -> Result<Vec<launchbox_game::Model>, DbErr> {
	let rows = launchbox_game::Entity::find()
		.join(
			JoinType::InnerJoin,
			launchbox_game::Relation::AlternateName.def(),
		)
		.filter(launchbox_game::Column::PlatformName.eq_ignore_case(platform_name))
		.filter(
			launchbox_game_alternate_name::Column::NameNormalized.eq_ignore_case(normalized_name),
		)
		.order_by(region_priority_case(prefer_regions), Order::Asc)
		.order_by_asc(launchbox_game_alternate_name::Column::Region)
		.limit(LB_MATCH_LIMIT)
		.all(conn)
		.await?;
	Ok(dedup_by_database_id(rows))
}

fn dedup_by_database_id(rows: Vec<launchbox_game::Model>) -> Vec<launchbox_game::Model> {
	let mut seen: HashSet<i64> = HashSet::with_capacity(rows.len());
	rows.into_iter()
		.filter(|m| seen.insert(m.database_id))
		.collect()
}

fn region_priority_case(prefer_regions: &[&str]) -> SimpleExpr {
	let region_col = Expr::col((
		launchbox_game_alternate_name::Entity,
		launchbox_game_alternate_name::Column::Region,
	));
	let prefer_owned: Vec<String> = prefer_regions.iter().map(|s| (*s).to_string()).collect();

	let mut case = CaseStatement::new();
	if !prefer_owned.is_empty() {
		case = case.case(region_col.clone().is_in(prefer_owned), 0_i32);
	}
	case.case(region_col.is_null(), 1_i32).finally(2_i32).into()
}

pub async fn find_lb_game_alternate_names(
	database_id: i64,
	conn: &DbConn,
) -> Result<Vec<launchbox_game_alternate_name::Model>, DbErr> {
	launchbox_game_alternate_name::Entity::find()
		.filter(launchbox_game_alternate_name::Column::LaunchboxGameDatabaseId.eq(database_id))
		.order_by_asc(launchbox_game_alternate_name::Column::Region)
		.all(conn)
		.await
}

pub async fn find_lb_game_images(
	database_id: i64,
	conn: &DbConn,
) -> Result<Vec<launchbox_game_image::Model>, DbErr> {
	launchbox_game_image::Entity::find()
		.filter(launchbox_game_image::Column::LaunchboxGameDatabaseId.eq(database_id))
		.order_by_asc(launchbox_game_image::Column::ImageType)
		.all(conn)
		.await
}

pub async fn find_lb_platform_by_name_lower(
	name: &str,
	conn: &DbConn,
) -> Result<Option<launchbox_platform::Model>, DbErr> {
	launchbox_platform::Entity::find()
		.filter(launchbox_platform::Column::Name.eq_ignore_case(name))
		.one(conn)
		.await
}

pub async fn latest_lb_import_md5(conn: &DbConn) -> Result<Option<String>, DbErr> {
	launchbox_import::Entity::find()
		.order_by_desc(launchbox_import::Column::ImportedAt)
		.one(conn)
		.await
		.map(|opt| opt.map(|m| m.md5))
}

pub async fn record_lb_import(
	md5: &str,
	game_count: i32,
	platform_count: i32,
	alternate_name_count: i32,
	image_count: i32,
	conn: &DbConn,
) -> Result<(), DbErr> {
	let row = launchbox_import::ActiveModel {
		md5: Set(md5.to_string()),
		game_count: Set(Some(game_count)),
		platform_count: Set(Some(platform_count)),
		alternate_name_count: Set(Some(alternate_name_count)),
		image_count: Set(Some(image_count)),
		..Default::default()
	};
	launchbox_import::Entity::insert(row)
		.on_conflict(
			OnConflict::column(launchbox_import::Column::Md5)
				.update_columns([
					launchbox_import::Column::ImportedAt,
					launchbox_import::Column::GameCount,
					launchbox_import::Column::PlatformCount,
					launchbox_import::Column::AlternateNameCount,
					launchbox_import::Column::ImageCount,
				])
				.to_owned(),
		)
		.exec(conn)
		.await?;
	Ok(())
}

pub async fn truncate_lb_alternate_names_and_images(conn: &DbConn) -> Result<(), DbErr> {
	launchbox_game_alternate_name::Entity::delete_many()
		.exec(conn)
		.await?;
	launchbox_game_image::Entity::delete_many()
		.exec(conn)
		.await?;
	Ok(())
}

pub async fn bulk_upsert_lb_platforms(
	rows: Vec<launchbox_platform::ActiveModel>,
	conn: &DbConn,
) -> Result<(), DbErr> {
	if rows.is_empty() {
		return Ok(());
	}
	launchbox_platform::Entity::insert_many(rows)
		.on_conflict(
			OnConflict::column(launchbox_platform::Column::Name)
				.update_columns([
					launchbox_platform::Column::Emulated,
					launchbox_platform::Column::ReleaseDate,
					launchbox_platform::Column::Developer,
					launchbox_platform::Column::Manufacturer,
					launchbox_platform::Column::Cpu,
					launchbox_platform::Column::Memory,
					launchbox_platform::Column::Graphics,
					launchbox_platform::Column::Sound,
					launchbox_platform::Column::Display,
					launchbox_platform::Column::Media,
					launchbox_platform::Column::MaxControllers,
					launchbox_platform::Column::Notes,
					launchbox_platform::Column::Category,
					launchbox_platform::Column::UpdatedAt,
				])
				.to_owned(),
		)
		.exec(conn)
		.await?;
	Ok(())
}

pub async fn bulk_upsert_lb_games(
	rows: Vec<launchbox_game::ActiveModel>,
	conn: &DbConn,
) -> Result<(), DbErr> {
	if rows.is_empty() {
		return Ok(());
	}
	launchbox_game::Entity::insert_many(rows)
		.on_conflict(
			OnConflict::column(launchbox_game::Column::DatabaseId)
				.update_columns([
					launchbox_game::Column::Name,
					launchbox_game::Column::NameNormalized,
					launchbox_game::Column::PlatformName,
					launchbox_game::Column::ReleaseDate,
					launchbox_game::Column::ReleaseYear,
					launchbox_game::Column::Overview,
					launchbox_game::Column::Developer,
					launchbox_game::Column::Publisher,
					launchbox_game::Column::Genres,
					launchbox_game::Column::MaxPlayers,
					launchbox_game::Column::Cooperative,
					launchbox_game::Column::Esrb,
					launchbox_game::Column::ReleaseType,
					launchbox_game::Column::Status,
					launchbox_game::Column::WikipediaUrl,
					launchbox_game::Column::VideoUrl,
					launchbox_game::Column::CommunityRating,
					launchbox_game::Column::CommunityRatingCount,
					launchbox_game::Column::UpdatedAt,
				])
				.to_owned(),
		)
		.exec(conn)
		.await?;
	Ok(())
}

pub async fn bulk_insert_lb_alternate_names(
	rows: Vec<launchbox_game_alternate_name::ActiveModel>,
	conn: &DbConn,
) -> Result<(), DbErr> {
	if rows.is_empty() {
		return Ok(());
	}
	launchbox_game_alternate_name::Entity::insert_many(rows)
		.exec(conn)
		.await?;
	Ok(())
}

pub async fn bulk_insert_lb_images(
	rows: Vec<launchbox_game_image::ActiveModel>,
	conn: &DbConn,
) -> Result<(), DbErr> {
	if rows.is_empty() {
		return Ok(());
	}
	launchbox_game_image::Entity::insert_many(rows)
		.exec(conn)
		.await?;
	Ok(())
}
