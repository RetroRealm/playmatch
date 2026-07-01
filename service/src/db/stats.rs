use chrono::{DateTime, Utc};
use entity::sea_orm_active_enums::MatchTypeEnum;
use entity::{
	company, dat_file, dat_file_import, game, game_file, platform, signature_group,
	signature_metadata_mapping,
};
use sea_orm::{
	ColumnTrait, DbConn, DbErr, EntityTrait, JoinType, PaginatorTrait, QueryFilter, QueryOrder,
	QuerySelect, RelationTrait,
};

/// Count rows in each reference and collection table plus the newest import time
/// in one round of queries. Used by the cacheable v2 service-wide stats endpoint.
pub struct GlobalCounts {
	pub dat_file_count: u64,
	pub signature_group_count: u64,
	pub platform_count: u64,
	pub company_count: u64,
	pub game_count: u64,
	pub current_game_count: u64,
	pub game_file_count: u64,
	pub mapped_game_count: u64,
	pub last_import_at: Option<DateTime<Utc>>,
}

/// A game is "mapped" when it carries at least one successful provider mapping
/// (an automatic or manual match). Failed and pending rows do not count.
fn mapped_game_count_query() -> sea_orm::Select<game::Entity> {
	game::Entity::find()
		.join(
			JoinType::InnerJoin,
			game::Relation::SignatureMetadataMapping.def(),
		)
		.filter(
			signature_metadata_mapping::Column::MatchType
				.is_in([MatchTypeEnum::Automatic, MatchTypeEnum::Manual]),
		)
		.select_only()
		.column(game::Column::Id)
		.group_by(game::Column::Id)
}

pub async fn collect_global_counts(conn: &DbConn) -> Result<GlobalCounts, DbErr> {
	let (
		dat_file_count,
		signature_group_count,
		platform_count,
		company_count,
		game_count,
		current_game_count,
		game_file_count,
		mapped_game_count,
		last_import_at,
	) = tokio::try_join!(
		dat_file::Entity::find().count(conn),
		signature_group::Entity::find().count(conn),
		platform::Entity::find().count(conn),
		company::Entity::find().count(conn),
		game::Entity::find().count(conn),
		game::Entity::find()
			.filter(game::Column::IsCurrent.eq(true))
			.count(conn),
		game_file::Entity::find().count(conn),
		mapped_game_count_query().count(conn),
		latest_import_at(conn),
	)?;

	Ok(GlobalCounts {
		dat_file_count,
		signature_group_count,
		platform_count,
		company_count,
		game_count,
		current_game_count,
		game_file_count,
		mapped_game_count,
		last_import_at,
	})
}

async fn latest_import_at(conn: &DbConn) -> Result<Option<DateTime<Utc>>, DbErr> {
	let latest = dat_file_import::Entity::find()
		.order_by_desc(dat_file_import::Column::ImportedAt)
		.one(conn)
		.await?;
	Ok(latest.map(|import| import.imported_at.into()))
}

pub struct PlatformCounts {
	pub dat_file_count: u64,
	pub current_game_count: u64,
	pub game_file_count: u64,
	pub mapped_game_count: u64,
}

pub async fn collect_platform_counts(
	platform_id: sea_orm::prelude::Uuid,
	conn: &DbConn,
) -> Result<PlatformCounts, DbErr> {
	let (dat_file_count, current_game_count, game_file_count, mapped_game_count) = tokio::try_join!(
		dat_file::Entity::find()
			.filter(dat_file::Column::PlatformId.eq(platform_id))
			.count(conn),
		games_for_platform(platform_id)
			.filter(game::Column::IsCurrent.eq(true))
			.count(conn),
		game_file::Entity::find()
			.join(JoinType::InnerJoin, game_file::Relation::Game.def())
			.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
			.join(
				JoinType::InnerJoin,
				dat_file_import::Relation::DatFile.def(),
			)
			.filter(dat_file::Column::PlatformId.eq(platform_id))
			.count(conn),
		mapped_games_for_platform(platform_id).count(conn),
	)?;

	Ok(PlatformCounts {
		dat_file_count,
		current_game_count,
		game_file_count,
		mapped_game_count,
	})
}

fn games_for_platform(platform_id: sea_orm::prelude::Uuid) -> sea_orm::Select<game::Entity> {
	game::Entity::find()
		.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
		.join(
			JoinType::InnerJoin,
			dat_file_import::Relation::DatFile.def(),
		)
		.filter(dat_file::Column::PlatformId.eq(platform_id))
}

fn mapped_games_for_platform(platform_id: sea_orm::prelude::Uuid) -> sea_orm::Select<game::Entity> {
	games_for_platform(platform_id)
		.join(
			JoinType::InnerJoin,
			game::Relation::SignatureMetadataMapping.def(),
		)
		.filter(
			signature_metadata_mapping::Column::MatchType
				.is_in([MatchTypeEnum::Automatic, MatchTypeEnum::Manual]),
		)
		.select_only()
		.column(game::Column::Id)
		.group_by(game::Column::Id)
}

/// Existence check used by the per-platform stats endpoint to answer 404 before
/// running the heavier count queries.
pub async fn platform_exists(
	platform_id: sea_orm::prelude::Uuid,
	conn: &DbConn,
) -> Result<bool, DbErr> {
	let one = platform::Entity::find()
		.filter(platform::Column::Id.eq(platform_id))
		.select_only()
		.column(platform::Column::Id)
		.into_tuple::<sea_orm::prelude::Uuid>()
		.one(conn)
		.await?;
	Ok(one.is_some())
}
