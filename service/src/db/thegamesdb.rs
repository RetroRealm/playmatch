use crate::db::abstraction::ColumnEqIgnoreCaseTrait;
use entity::{tgdb_game, tgdb_game_alias};
use sea_orm::ActiveValue::Set;
use sea_orm::sea_query::OnConflict;
use sea_orm::{
	ColumnTrait, DbConn, DbErr, EntityTrait, JoinType, QueryFilter, QueryOrder, QuerySelect,
	RelationTrait,
};
use std::collections::HashSet;

const TGDB_MATCH_LIMIT: u64 = 50;

pub async fn find_tgdb_game_by_id(
	id: i64,
	conn: &DbConn,
) -> Result<Option<tgdb_game::Model>, DbErr> {
	tgdb_game::Entity::find_by_id(id).one(conn).await
}

pub async fn find_tgdb_games_by_title(
	title: &str,
	conn: &DbConn,
) -> Result<Vec<tgdb_game::Model>, DbErr> {
	tgdb_game::Entity::find()
		.filter(tgdb_game::Column::Title.eq_ignore_case(title))
		.limit(TGDB_MATCH_LIMIT)
		.all(conn)
		.await
}

pub async fn find_tgdb_games_by_title_normalized(
	normalized: &str,
	conn: &DbConn,
) -> Result<Vec<tgdb_game::Model>, DbErr> {
	tgdb_game::Entity::find()
		.filter(tgdb_game::Column::TitleNormalized.eq_ignore_case(normalized))
		.limit(TGDB_MATCH_LIMIT)
		.all(conn)
		.await
}

pub async fn find_tgdb_games_by_alias_name(
	alt_name: &str,
	conn: &DbConn,
) -> Result<Vec<tgdb_game::Model>, DbErr> {
	let rows = tgdb_game::Entity::find()
		.join(JoinType::InnerJoin, tgdb_game::Relation::Alias.def())
		.filter(tgdb_game_alias::Column::AltName.eq_ignore_case(alt_name))
		.order_by_asc(tgdb_game::Column::Id)
		.limit(TGDB_MATCH_LIMIT)
		.all(conn)
		.await?;
	Ok(dedup_by_id(rows))
}

pub async fn find_tgdb_games_by_alias_name_normalized(
	normalized: &str,
	conn: &DbConn,
) -> Result<Vec<tgdb_game::Model>, DbErr> {
	let rows = tgdb_game::Entity::find()
		.join(JoinType::InnerJoin, tgdb_game::Relation::Alias.def())
		.filter(tgdb_game_alias::Column::AltNameNormalized.eq_ignore_case(normalized))
		.order_by_asc(tgdb_game::Column::Id)
		.limit(TGDB_MATCH_LIMIT)
		.all(conn)
		.await?;
	Ok(dedup_by_id(rows))
}

pub async fn find_tgdb_aliases_for_game(
	game_id: i64,
	conn: &DbConn,
) -> Result<Vec<tgdb_game_alias::Model>, DbErr> {
	tgdb_game_alias::Entity::find()
		.filter(tgdb_game_alias::Column::GameId.eq(game_id))
		.order_by_asc(tgdb_game_alias::Column::AltName)
		.all(conn)
		.await
}

/// Insert a TGDB game returned by the API search rung so future cycles can hit
/// the local rungs for the same id. `ON CONFLICT DO NOTHING` covers the race
/// where two parallel API rungs upsert the same id.
pub async fn upsert_tgdb_game(
	id: i64,
	title: &str,
	title_normalized: Option<&str>,
	conn: &DbConn,
) -> Result<(), DbErr> {
	let row = tgdb_game::ActiveModel {
		id: Set(id),
		title: Set(title.to_string()),
		title_normalized: Set(title_normalized.map(|s| s.to_string())),
		..Default::default()
	};
	tgdb_game::Entity::insert(row)
		.on_conflict(
			OnConflict::column(tgdb_game::Column::Id)
				.do_nothing()
				.to_owned(),
		)
		.do_nothing()
		.exec(conn)
		.await?;
	Ok(())
}

pub async fn insert_tgdb_aliases(
	game_id: i64,
	aliases: &[(String, Option<String>)],
	conn: &DbConn,
) -> Result<(), DbErr> {
	if aliases.is_empty() {
		return Ok(());
	}
	let rows: Vec<tgdb_game_alias::ActiveModel> = aliases
		.iter()
		.map(|(alt, norm)| tgdb_game_alias::ActiveModel {
			game_id: Set(game_id),
			alt_name: Set(alt.clone()),
			alt_name_normalized: Set(norm.clone()),
			..Default::default()
		})
		.collect();
	tgdb_game_alias::Entity::insert_many(rows)
		.exec(conn)
		.await?;
	Ok(())
}

fn dedup_by_id(rows: Vec<tgdb_game::Model>) -> Vec<tgdb_game::Model> {
	let mut seen: HashSet<i64> = HashSet::with_capacity(rows.len());
	rows.into_iter().filter(|m| seen.insert(m.id)).collect()
}
