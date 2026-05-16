use crate::db::abstraction::ColumnEqIgnoreCaseTrait;
use entity::{
	retroachievements_game, retroachievements_game_hash, retroachievements_import,
	retroachievements_system,
};
use sea_orm::ActiveValue::Set;
use sea_orm::sea_query::OnConflict;
use sea_orm::{
	ColumnTrait, DbConn, DbErr, EntityTrait, JoinType, Order, QueryFilter, QueryOrder, QuerySelect,
	RelationTrait,
};

const RA_SEARCH_LIMIT: u64 = 50;
const RA_MATCH_LIMIT: u64 = 25;

pub async fn list_retroachievements_systems(
	conn: &DbConn,
) -> Result<Vec<retroachievements_system::Model>, DbErr> {
	retroachievements_system::Entity::find()
		.order_by_asc(retroachievements_system::Column::Name)
		.all(conn)
		.await
}

pub async fn find_retroachievements_system_by_name_lower(
	name: &str,
	conn: &DbConn,
) -> Result<Option<retroachievements_system::Model>, DbErr> {
	retroachievements_system::Entity::find()
		.filter(retroachievements_system::Column::Name.eq_ignore_case(name))
		.one(conn)
		.await
}

pub async fn find_retroachievements_game_by_id(
	game_id: i64,
	conn: &DbConn,
) -> Result<Option<retroachievements_game::Model>, DbErr> {
	retroachievements_game::Entity::find()
		.filter(retroachievements_game::Column::GameId.eq(game_id))
		.one(conn)
		.await
}

/// Look up a game by one of its MD5 hashes. The `lower(md5)` functional index
/// keeps this index-only.
pub async fn find_retroachievements_game_by_md5(
	md5: &str,
	conn: &DbConn,
) -> Result<Option<retroachievements_game::Model>, DbErr> {
	retroachievements_game::Entity::find()
		.join(
			JoinType::InnerJoin,
			retroachievements_game::Relation::Hash.def(),
		)
		.filter(retroachievements_game_hash::Column::Md5.eq_ignore_case(md5))
		.one(conn)
		.await
}

pub async fn find_retroachievements_games_by_system_and_title(
	system_name: &str,
	title: &str,
	conn: &DbConn,
) -> Result<Vec<retroachievements_game::Model>, DbErr> {
	retroachievements_game::Entity::find()
		.filter(
			retroachievements_game::Column::SystemName
				.eq_ignore_case(system_name)
				.and(retroachievements_game::Column::Title.eq_ignore_case(title)),
		)
		.limit(RA_MATCH_LIMIT)
		.all(conn)
		.await
}

pub async fn find_retroachievements_games_by_system_and_title_normalized(
	system_name: &str,
	title_normalized: &str,
	conn: &DbConn,
) -> Result<Vec<retroachievements_game::Model>, DbErr> {
	retroachievements_game::Entity::find()
		.filter(
			retroachievements_game::Column::SystemName
				.eq_ignore_case(system_name)
				.and(
					retroachievements_game::Column::TitleNormalized
						.eq_ignore_case(title_normalized),
				),
		)
		.limit(RA_MATCH_LIMIT)
		.all(conn)
		.await
}

pub async fn search_retroachievements_games(
	system_name: Option<&str>,
	query: &str,
	conn: &DbConn,
) -> Result<Vec<retroachievements_game::Model>, DbErr> {
	let pattern = format!("%{}%", query.to_lowercase());
	let mut q = retroachievements_game::Entity::find().filter(
		sea_orm::sea_query::Expr::expr(sea_orm::sea_query::Func::lower(
			sea_orm::sea_query::Expr::col(retroachievements_game::Column::Title),
		))
		.like(pattern),
	);
	if let Some(s) = system_name {
		q = q.filter(retroachievements_game::Column::SystemName.eq_ignore_case(s));
	}
	q.order_by(retroachievements_game::Column::Title, Order::Asc)
		.limit(RA_SEARCH_LIMIT)
		.all(conn)
		.await
}

pub async fn find_retroachievements_hashes_for_game(
	game_id: i64,
	conn: &DbConn,
) -> Result<Vec<retroachievements_game_hash::Model>, DbErr> {
	retroachievements_game_hash::Entity::find()
		.filter(retroachievements_game_hash::Column::GameId.eq(game_id))
		.order_by_asc(retroachievements_game_hash::Column::Md5)
		.all(conn)
		.await
}

pub async fn latest_retroachievements_import_at(
	conn: &DbConn,
) -> Result<Option<retroachievements_import::Model>, DbErr> {
	retroachievements_import::Entity::find()
		.order_by_desc(retroachievements_import::Column::ImportedAt)
		.one(conn)
		.await
}

pub async fn record_retroachievements_import(
	system_count: i32,
	game_count: i32,
	hash_count: i32,
	conn: &DbConn,
) -> Result<(), DbErr> {
	let row = retroachievements_import::ActiveModel {
		system_count: Set(Some(system_count)),
		game_count: Set(Some(game_count)),
		hash_count: Set(Some(hash_count)),
		..Default::default()
	};
	retroachievements_import::Entity::insert(row)
		.exec(conn)
		.await?;
	Ok(())
}

/// Per-cycle reset of games and hashes. Systems persist across cycles so any
/// platform mappings that target a `provider_id = system_id` survive.
pub async fn truncate_retroachievements_games_and_hashes(conn: &DbConn) -> Result<(), DbErr> {
	retroachievements_game_hash::Entity::delete_many()
		.exec(conn)
		.await?;
	retroachievements_game::Entity::delete_many()
		.exec(conn)
		.await?;
	Ok(())
}

pub async fn bulk_upsert_retroachievements_systems(
	rows: Vec<retroachievements_system::ActiveModel>,
	conn: &DbConn,
) -> Result<(), DbErr> {
	if rows.is_empty() {
		return Ok(());
	}
	retroachievements_system::Entity::insert_many(rows)
		.on_conflict(
			OnConflict::column(retroachievements_system::Column::SystemId)
				.update_columns([
					retroachievements_system::Column::Name,
					retroachievements_system::Column::UpdatedAt,
				])
				.to_owned(),
		)
		.exec(conn)
		.await?;
	Ok(())
}

pub async fn bulk_upsert_retroachievements_games(
	rows: Vec<retroachievements_game::ActiveModel>,
	conn: &DbConn,
) -> Result<(), DbErr> {
	if rows.is_empty() {
		return Ok(());
	}
	retroachievements_game::Entity::insert_many(rows)
		.on_conflict(
			OnConflict::column(retroachievements_game::Column::GameId)
				.update_columns([
					retroachievements_game::Column::SystemId,
					retroachievements_game::Column::SystemName,
					retroachievements_game::Column::Title,
					retroachievements_game::Column::TitleNormalized,
					retroachievements_game::Column::ImageIcon,
					retroachievements_game::Column::NumAchievements,
					retroachievements_game::Column::NumLeaderboards,
					retroachievements_game::Column::Points,
					retroachievements_game::Column::DateModified,
					retroachievements_game::Column::ForumTopicId,
					retroachievements_game::Column::UpdatedAt,
				])
				.to_owned(),
		)
		.exec(conn)
		.await?;
	Ok(())
}

pub async fn bulk_insert_retroachievements_hashes(
	rows: Vec<retroachievements_game_hash::ActiveModel>,
	conn: &DbConn,
) -> Result<(), DbErr> {
	if rows.is_empty() {
		return Ok(());
	}
	retroachievements_game_hash::Entity::insert_many(rows)
		.exec(conn)
		.await?;
	Ok(())
}
