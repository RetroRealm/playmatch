use crate::error;
use crate::model::openvgdb::{OvgdbHashQuery, OvgdbReleaseIdQuery};
use actix_web::web::Data;
use actix_web::{HttpResponse, Responder, get};
use actix_web_lab::extract::Query;
use redis::aio::MultiplexedConnection;
use sea_orm::DatabaseConnection;
use service::providers::openvgdb::cache::{
	HashKind, get_ovgdb_release_by_id_cached, get_ovgdb_releases_for_rom_cached,
	get_ovgdb_rom_by_hash_cached,
};
use service::providers::openvgdb::model::OvgdbRomMatch;
#[allow(unused_imports)] // Referenced only inside utoipa::path body attributes.
use service::providers::openvgdb::model::{OvgdbRelease, OvgdbRom};

/// Returns an OpenVGDB release by id.
#[utoipa::path(
	get,
	tag = "OpenVGDB",
	params(OvgdbReleaseIdQuery),
	responses(
		(status = 200, description = "The matched OpenVGDB release record", body = OvgdbRelease),
		(status = 404, description = "Release not found")
	)
)]
#[get("/openvgdb/release")]
pub async fn get_ovgdb_release_by_id(
	query: Query<OvgdbReleaseIdQuery>,
	db_conn: Data<DatabaseConnection>,
	redis_conn: Data<MultiplexedConnection>,
) -> error::Result<impl Responder> {
	let mut redis_conn = redis_conn.get_ref().clone();
	let response = get_ovgdb_release_by_id_cached(
		db_conn.get_ref(),
		&mut redis_conn,
		query.into_inner().release_id,
	)
	.await?;
	Ok(match response {
		Some(release) => HttpResponse::Ok().json(release),
		None => HttpResponse::NotFound().finish(),
	})
}

/// Looks up an OpenVGDB rom by a file hash.
///
/// The strongest supplied hash resolves the rom: sha1, then md5, then crc. At least one hash must be supplied. The response carries the matched rom and every release attached to it.
#[utoipa::path(
	get,
	tag = "OpenVGDB",
	params(OvgdbHashQuery),
	responses(
		(status = 200, description = "The matched rom and its releases", body = OvgdbRomMatch),
		(status = 400, description = "No hash supplied"),
		(status = 404, description = "No rom matched the supplied hash")
	)
)]
#[get("/openvgdb/rom/by-hash")]
pub async fn get_ovgdb_rom_by_hash(
	query: Query<OvgdbHashQuery>,
	db_conn: Data<DatabaseConnection>,
	redis_conn: Data<MultiplexedConnection>,
) -> error::Result<impl Responder> {
	let q = query.into_inner();
	let mut redis_conn = redis_conn.get_ref().clone();

	let lookup = q
		.sha1
		.filter(|s| !s.trim().is_empty())
		.map(|sha1| (HashKind::Sha1, sha1))
		.or_else(|| {
			q.md5
				.filter(|s| !s.trim().is_empty())
				.map(|md5| (HashKind::Md5, md5))
		})
		.or_else(|| {
			q.crc
				.filter(|s| !s.trim().is_empty())
				.map(|crc| (HashKind::Crc, crc))
		});

	let Some((kind, hash)) = lookup else {
		return Ok(HttpResponse::BadRequest().body("supply at least one of sha1, md5 or crc"));
	};

	let rom = get_ovgdb_rom_by_hash_cached(db_conn.get_ref(), &mut redis_conn, kind, hash).await?;
	let Some(rom) = rom else {
		return Ok(HttpResponse::NotFound().finish());
	};

	let releases =
		get_ovgdb_releases_for_rom_cached(db_conn.get_ref(), &mut redis_conn, rom.rom_id).await?;

	Ok(HttpResponse::Ok().json(OvgdbRomMatch { rom, releases }))
}
