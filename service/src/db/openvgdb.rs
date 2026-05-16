use crate::db::abstraction::ColumnEqIgnoreCaseTrait;
use entity::{openvgdb_import, openvgdb_release, openvgdb_rom};
use sea_orm::ActiveValue::Set;
use sea_orm::sea_query::{CaseStatement, Expr, OnConflict, SimpleExpr};
use sea_orm::{
	ColumnTrait, DbConn, DbErr, EntityTrait, Order, QueryFilter, QueryOrder, QuerySelect,
};

const OVGDB_RELEASE_LIST_LIMIT: u64 = 50;
/// Bounds the candidate set returned by the title-name match helpers used by
/// the matcher's name fallback rungs.
const OVGDB_TITLE_MATCH_LIMIT: u64 = 25;

pub async fn find_openvgdb_rom_by_sha1(
	sha1: &str,
	conn: &DbConn,
) -> Result<Option<openvgdb_rom::Model>, DbErr> {
	openvgdb_rom::Entity::find()
		.filter(openvgdb_rom::Column::RomHashSha1.eq_ignore_case(sha1))
		.one(conn)
		.await
}

pub async fn find_openvgdb_rom_by_md5(
	md5: &str,
	conn: &DbConn,
) -> Result<Option<openvgdb_rom::Model>, DbErr> {
	openvgdb_rom::Entity::find()
		.filter(openvgdb_rom::Column::RomHashMd5.eq_ignore_case(md5))
		.one(conn)
		.await
}

pub async fn find_openvgdb_rom_by_crc(
	crc: &str,
	conn: &DbConn,
) -> Result<Option<openvgdb_rom::Model>, DbErr> {
	openvgdb_rom::Entity::find()
		.filter(openvgdb_rom::Column::RomHashCrc.eq_ignore_case(crc))
		.one(conn)
		.await
}

/// Case-insensitive direct title-name lookup, ordered by region preference
/// then alphabetical. Used by the matcher's name fallback rung when hash
/// matching has missed.
pub async fn find_openvgdb_releases_by_title_lower(
	title: &str,
	prefer_regions: &[&str],
	conn: &DbConn,
) -> Result<Vec<openvgdb_release::Model>, DbErr> {
	openvgdb_release::Entity::find()
		.filter(openvgdb_release::Column::TitleName.eq_ignore_case(title))
		.order_by(region_priority_case(prefer_regions), Order::Asc)
		.order_by_asc(openvgdb_release::Column::RegionName)
		.limit(OVGDB_TITLE_MATCH_LIMIT)
		.all(conn)
		.await
}

/// Case-insensitive normalized title-name lookup. Skips rows whose
/// `title_name_normalized` is NULL via the implicit eq_ignore_case match
/// (NULL never equals a non-NULL string), so existing rows that pre-date
/// the column population stay invisible to this rung until the next
/// OpenVGDB import refresh fills them in.
pub async fn find_openvgdb_releases_by_title_normalized(
	normalized: &str,
	prefer_regions: &[&str],
	conn: &DbConn,
) -> Result<Vec<openvgdb_release::Model>, DbErr> {
	openvgdb_release::Entity::find()
		.filter(openvgdb_release::Column::TitleNameNormalized.eq_ignore_case(normalized))
		.order_by(region_priority_case(prefer_regions), Order::Asc)
		.order_by_asc(openvgdb_release::Column::RegionName)
		.limit(OVGDB_TITLE_MATCH_LIMIT)
		.all(conn)
		.await
}

/// Releases attached to one OpenVGDB rom, ordered so that rows whose
/// `region_name` is in `prefer_regions` come first, then NULL regions, then
/// the rest.
pub async fn find_openvgdb_releases_for_rom(
	rom_id: i64,
	prefer_regions: &[&str],
	conn: &DbConn,
) -> Result<Vec<openvgdb_release::Model>, DbErr> {
	openvgdb_release::Entity::find()
		.filter(openvgdb_release::Column::RomId.eq(rom_id))
		.order_by(region_priority_case(prefer_regions), Order::Asc)
		.order_by_asc(openvgdb_release::Column::RegionName)
		.all(conn)
		.await
}

fn region_priority_case(prefer_regions: &[&str]) -> SimpleExpr {
	let region_col = Expr::col((
		openvgdb_release::Entity,
		openvgdb_release::Column::RegionName,
	));
	let prefer_owned: Vec<String> = prefer_regions.iter().map(|s| (*s).to_string()).collect();

	let mut case = CaseStatement::new();
	if !prefer_owned.is_empty() {
		case = case.case(region_col.clone().is_in(prefer_owned), 0_i32);
	}
	case.case(region_col.is_null(), 1_i32).finally(2_i32).into()
}

pub async fn find_openvgdb_release_by_id(
	release_id: i64,
	conn: &DbConn,
) -> Result<Option<openvgdb_release::Model>, DbErr> {
	openvgdb_release::Entity::find()
		.filter(openvgdb_release::Column::ReleaseId.eq(release_id))
		.one(conn)
		.await
}

pub async fn list_openvgdb_releases_for_rom_id(
	rom_id: i64,
	conn: &DbConn,
) -> Result<Vec<openvgdb_release::Model>, DbErr> {
	openvgdb_release::Entity::find()
		.filter(openvgdb_release::Column::RomId.eq(rom_id))
		.order_by_asc(openvgdb_release::Column::RegionName)
		.limit(OVGDB_RELEASE_LIST_LIMIT)
		.all(conn)
		.await
}

pub async fn latest_openvgdb_import_md5(conn: &DbConn) -> Result<Option<String>, DbErr> {
	openvgdb_import::Entity::find()
		.order_by_desc(openvgdb_import::Column::ImportedAt)
		.one(conn)
		.await
		.map(|opt| opt.map(|m| m.md5))
}

pub async fn record_openvgdb_import(
	md5: &str,
	rom_count: i32,
	release_count: i32,
	conn: &DbConn,
) -> Result<(), DbErr> {
	let row = openvgdb_import::ActiveModel {
		md5: Set(md5.to_string()),
		rom_count: Set(Some(rom_count)),
		release_count: Set(Some(release_count)),
		..Default::default()
	};
	openvgdb_import::Entity::insert(row)
		.on_conflict(
			OnConflict::column(openvgdb_import::Column::Md5)
				.update_columns([
					openvgdb_import::Column::ImportedAt,
					openvgdb_import::Column::RomCount,
					openvgdb_import::Column::ReleaseCount,
				])
				.to_owned(),
		)
		.exec(conn)
		.await?;
	Ok(())
}

pub async fn bulk_upsert_openvgdb_roms(
	rows: Vec<openvgdb_rom::ActiveModel>,
	conn: &DbConn,
) -> Result<(), DbErr> {
	if rows.is_empty() {
		return Ok(());
	}
	openvgdb_rom::Entity::insert_many(rows)
		.on_conflict(
			OnConflict::column(openvgdb_rom::Column::RomId)
				.update_columns([
					openvgdb_rom::Column::SystemId,
					openvgdb_rom::Column::RegionId,
					openvgdb_rom::Column::RomHashCrc,
					openvgdb_rom::Column::RomHashMd5,
					openvgdb_rom::Column::RomHashSha1,
					openvgdb_rom::Column::RomSize,
					openvgdb_rom::Column::RomFileName,
					openvgdb_rom::Column::RomExtensionlessFileName,
					openvgdb_rom::Column::RomSerial,
					openvgdb_rom::Column::UpdatedAt,
				])
				.to_owned(),
		)
		.exec(conn)
		.await?;
	Ok(())
}

pub async fn bulk_upsert_openvgdb_releases(
	rows: Vec<openvgdb_release::ActiveModel>,
	conn: &DbConn,
) -> Result<(), DbErr> {
	if rows.is_empty() {
		return Ok(());
	}
	openvgdb_release::Entity::insert_many(rows)
		.on_conflict(
			OnConflict::column(openvgdb_release::Column::ReleaseId)
				.update_columns([
					openvgdb_release::Column::RomId,
					openvgdb_release::Column::TitleName,
					openvgdb_release::Column::TitleNameNormalized,
					openvgdb_release::Column::RegionName,
					openvgdb_release::Column::SystemName,
					openvgdb_release::Column::CoverFront,
					openvgdb_release::Column::CoverBack,
					openvgdb_release::Column::Description,
					openvgdb_release::Column::Developer,
					openvgdb_release::Column::Publisher,
					openvgdb_release::Column::Genre,
					openvgdb_release::Column::ReleaseDate,
					openvgdb_release::Column::ReleaseYear,
					openvgdb_release::Column::ReferenceUrl,
				])
				.to_owned(),
		)
		.exec(conn)
		.await?;
	Ok(())
}
