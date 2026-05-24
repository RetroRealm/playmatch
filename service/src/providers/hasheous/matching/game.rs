use crate::db::game::{
	get_automatic_match_failed_games_with_limit,
	get_unmatched_games_with_clone_of_with_limit_no_platform_gate,
	get_unmatched_games_without_clone_of_with_limit_no_platform_gate,
};
use crate::db::game_file::get_game_files_from_game_id;
use crate::providers::MetadataProvider;
use crate::providers::hasheous::HashLookupOutcome;
use crate::providers::hasheous::HasheousClient;
use crate::providers::hasheous::model::{HashLookupRequest, HashLookupResponse, HasheousGame};
use crate::providers::{
	Target, drive_match_pipeline, write_auto_match_failed, write_auto_match_success,
};
use entity::game::Model;
use entity::sea_orm_active_enums::{
	AutomaticMatchReasonEnum, FailedMatchReasonEnum, MetadataProviderEnum,
};
use futures_util::future::BoxFuture;
use log::debug;
use sea_orm::DbConn;
use std::sync::Arc;

pub async fn match_games_to_hasheous(
	client: Arc<HasheousClient>,
	db_conn: &DbConn,
) -> anyhow::Result<()> {
	let chunk_size = client.chunk_size();
	drive_match_pipeline(
		"game",
		MetadataProviderEnum::Hasheous,
		get_unmatched_games_without_clone_of_with_limit_no_platform_gate,
		match_game_to_hasheous,
		client.clone(),
		db_conn,
		chunk_size,
	)
	.await?;
	debug!("Finished matching games without clone_of id to Hasheous");

	drive_match_pipeline(
		"game",
		MetadataProviderEnum::Hasheous,
		get_unmatched_games_with_clone_of_with_limit_no_platform_gate,
		match_clone_of_game_to_hasheous,
		client.clone(),
		db_conn,
		chunk_size,
	)
	.await?;
	debug!("Finished matching games with clone_of id to Hasheous");

	drive_match_pipeline(
		"game",
		MetadataProviderEnum::Hasheous,
		get_automatic_match_failed_games_with_limit,
		match_game_to_hasheous,
		client,
		db_conn,
		chunk_size,
	)
	.await?;
	debug!("Finished retrying previously failed Hasheous game matches");

	Ok(())
}

fn match_clone_of_game_to_hasheous(
	game: Model,
	client: Arc<HasheousClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(crate::providers::drive_clone_propagation(
		game,
		client,
		db_conn,
		match_game_to_hasheous,
	))
}

/// `Ord` derive is load-bearing: variants are listed weakest to strongest so
/// `pick_winning_hash_kind` can resolve which hash matched via `.max()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum HashKind {
	Crc,
	Md5,
	Sha1,
	Sha256,
}

fn match_game_to_hasheous(
	game: Model,
	client: Arc<HasheousClient>,
	db_conn: DbConn,
) -> BoxFuture<'static, anyhow::Result<()>> {
	Box::pin(async move {
		let mut redis_conn = client.redis_conn().clone();
		let files = get_game_files_from_game_id(game.id, &db_conn).await?;

		if files.is_empty() {
			write_auto_match_failed(
				"hasheous",
				MetadataProviderEnum::Hasheous,
				Target::Game(game.id),
				FailedMatchReasonEnum::NoDirectMatch,
				&db_conn,
				&mut redis_conn,
			)
			.await?;
			return Ok(());
		}

		for file in &files {
			let body = HashLookupRequest {
				md5: clean_hash(file.md5.as_deref()),
				sha1: clean_hash(file.sha1.as_deref()),
				sha256: clean_hash(file.sha256.as_deref()),
				crc: clean_hash(file.crc.as_deref()),
			};
			if body.is_empty() {
				continue;
			}
			let submitted = submitted_hashes(&body);

			match client.lookup_by_hash(&body).await? {
				HashLookupOutcome::Hit(resp) => {
					let winning = pick_winning_hash_kind(&resp, &body, &submitted);
					for h in &submitted {
						let outcome = if *h == winning { "hit" } else { "miss" };
						crate::metrics::record_match_rung("hasheous", rung_for(*h), outcome);
					}
					let matched_name = picked_name(&resp);
					let matched_year = picked_year(&resp);
					debug!(
						"Matched Game \"{}\" to Hasheous Game ID {} ({:?})",
						&game.name, resp.id, winning
					);
					write_auto_match_success(
						"hasheous",
						MetadataProviderEnum::Hasheous,
						Target::Game(game.id),
						resp.id.to_string(),
						reason_for(winning),
						matched_name,
						matched_year,
						&db_conn,
						&mut redis_conn,
					)
					.await?;
					return Ok(());
				}
				HashLookupOutcome::NotFound => {
					for h in &submitted {
						crate::metrics::record_match_rung("hasheous", rung_for(*h), "miss");
					}
				}
			}
		}

		debug!("No Hasheous match found for Game \"{}\"", &game.name);
		write_auto_match_failed(
			"hasheous",
			MetadataProviderEnum::Hasheous,
			Target::Game(game.id),
			FailedMatchReasonEnum::NoDirectMatch,
			&db_conn,
			&mut redis_conn,
		)
		.await?;
		Ok(())
	})
}

fn clean_hash(value: Option<&str>) -> Option<String> {
	let v = value?.trim();
	if v.is_empty() {
		return None;
	}
	Some(v.to_owned())
}

fn submitted_hashes(body: &HashLookupRequest) -> Vec<HashKind> {
	let mut out = Vec::with_capacity(4);
	if body.sha256.is_some() {
		out.push(HashKind::Sha256);
	}
	if body.sha1.is_some() {
		out.push(HashKind::Sha1);
	}
	if body.md5.is_some() {
		out.push(HashKind::Md5);
	}
	if body.crc.is_some() {
		out.push(HashKind::Crc);
	}
	out
}

fn rung_for(kind: HashKind) -> &'static str {
	match kind {
		HashKind::Sha256 => "sha256_hash",
		HashKind::Sha1 => "sha1_hash",
		HashKind::Md5 => "md5_hash",
		HashKind::Crc => "crc_hash",
	}
}

// SHA256 is not yet a variant on AutomaticMatchReasonEnum; record it as Sha1Hash
// for now so we do not need to grow the enum in this change.
fn reason_for(kind: HashKind) -> AutomaticMatchReasonEnum {
	match kind {
		HashKind::Sha256 | HashKind::Sha1 => AutomaticMatchReasonEnum::Sha1Hash,
		HashKind::Md5 => AutomaticMatchReasonEnum::Md5Hash,
		HashKind::Crc => AutomaticMatchReasonEnum::CrcHash,
	}
}

/// Hasheous's response carries the matched signature(s) but does not flag
/// which submitted hash drove the match. Walk the response's first signature
/// rom and resolve to the strongest hash that appears on both sides. Falls
/// back to the strongest submitted hash if nothing aligns.
fn pick_winning_hash_kind(
	resp: &HashLookupResponse,
	body: &HashLookupRequest,
	submitted: &[HashKind],
) -> HashKind {
	let strongest_submitted = *submitted
		.iter()
		.max()
		.expect("submitted must be non-empty when pick_winning_hash_kind is called");

	let Some(rom) = resp.signature.as_ref().and_then(|s| {
		s.rom
			.as_ref()
			.or_else(|| s.game.as_ref().and_then(|g| g.roms.as_ref()?.first()))
	}) else {
		return strongest_submitted;
	};

	let mut best: Option<HashKind> = None;
	let mut consider = |kind: HashKind| {
		if best.map(|b| kind > b).unwrap_or(true) {
			best = Some(kind);
		}
	};
	if let (Some(want), Some(got)) = (body.sha256.as_deref(), rom.sha256.as_deref())
		&& want.eq_ignore_ascii_case(got)
	{
		consider(HashKind::Sha256);
	}
	if let (Some(want), Some(got)) = (body.sha1.as_deref(), rom.sha1.as_deref())
		&& want.eq_ignore_ascii_case(got)
	{
		consider(HashKind::Sha1);
	}
	if let (Some(want), Some(got)) = (body.md5.as_deref(), rom.md5.as_deref())
		&& want.eq_ignore_ascii_case(got)
	{
		consider(HashKind::Md5);
	}
	if let (Some(want), Some(got)) = (body.crc.as_deref(), rom.crc.as_deref())
		&& want.eq_ignore_ascii_case(got)
	{
		consider(HashKind::Crc);
	}
	best.unwrap_or(strongest_submitted)
}

fn picked_name(resp: &HashLookupResponse) -> Option<String> {
	if let Some(g) = resp
		.signature
		.as_ref()
		.and_then(|s| s.game.as_ref())
		.and_then(|g: &HasheousGame| g.name.as_deref())
	{
		return Some(g.to_owned());
	}
	resp.name.clone()
}

fn picked_year(resp: &HashLookupResponse) -> Option<i16> {
	let raw = resp
		.signature
		.as_ref()
		.and_then(|s| s.game.as_ref())
		.and_then(|g| g.year.as_deref())?;
	// Hasheous year strings are sometimes "1990", "199x", "1990-01-01"; take the
	// leading 4 digits if they look like a calendar year.
	let head: String = raw.chars().take_while(char::is_ascii_digit).collect();
	if head.len() != 4 {
		return None;
	}
	head.parse::<i16>().ok()
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::providers::hasheous::model::{HasheousRom, SignatureLookupItem};

	fn rom_with(sha1: Option<&str>, md5: Option<&str>, crc: Option<&str>) -> HasheousRom {
		HasheousRom {
			md5: md5.map(str::to_owned),
			sha1: sha1.map(str::to_owned),
			sha256: None,
			crc: crc.map(str::to_owned),
			name: None,
			size: None,
		}
	}

	fn resp_with_rom(rom: HasheousRom) -> HashLookupResponse {
		HashLookupResponse {
			id: 1,
			name: None,
			platform: None,
			publisher: None,
			signature: Some(SignatureLookupItem {
				game: None,
				rom: Some(rom),
			}),
			metadata: None,
		}
	}

	#[test]
	fn picks_strongest_matching_hash_kind() {
		let body = HashLookupRequest {
			sha1: Some("AAA".into()),
			md5: Some("BBB".into()),
			crc: Some("CCC".into()),
			..Default::default()
		};
		let submitted = submitted_hashes(&body);
		let resp = resp_with_rom(rom_with(Some("aaa"), Some("bbb"), Some("ccc")));
		assert_eq!(
			pick_winning_hash_kind(&resp, &body, &submitted),
			HashKind::Sha1
		);
	}

	#[test]
	fn picks_md5_when_only_md5_matches() {
		let body = HashLookupRequest {
			sha1: Some("AAA".into()),
			md5: Some("BBB".into()),
			..Default::default()
		};
		let submitted = submitted_hashes(&body);
		let resp = resp_with_rom(rom_with(Some("different"), Some("bbb"), None));
		assert_eq!(
			pick_winning_hash_kind(&resp, &body, &submitted),
			HashKind::Md5
		);
	}

	#[test]
	fn falls_back_to_strongest_submitted_when_rom_absent() {
		let body = HashLookupRequest {
			sha1: Some("AAA".into()),
			crc: Some("CCC".into()),
			..Default::default()
		};
		let submitted = submitted_hashes(&body);
		let resp = HashLookupResponse {
			id: 1,
			name: None,
			platform: None,
			publisher: None,
			signature: None,
			metadata: None,
		};
		assert_eq!(
			pick_winning_hash_kind(&resp, &body, &submitted),
			HashKind::Sha1
		);
	}

	#[test]
	fn submitted_hashes_orders_strongest_first() {
		let body = HashLookupRequest {
			md5: Some("a".into()),
			sha1: Some("b".into()),
			sha256: Some("c".into()),
			crc: Some("d".into()),
		};
		let submitted = submitted_hashes(&body);
		assert_eq!(submitted[0], HashKind::Sha256);
		assert_eq!(*submitted.iter().max().unwrap(), HashKind::Sha256);
	}

	#[test]
	fn reason_for_maps_sha256_to_sha1_reason() {
		assert_eq!(
			reason_for(HashKind::Sha256),
			AutomaticMatchReasonEnum::Sha1Hash
		);
		assert_eq!(
			reason_for(HashKind::Sha1),
			AutomaticMatchReasonEnum::Sha1Hash
		);
		assert_eq!(reason_for(HashKind::Md5), AutomaticMatchReasonEnum::Md5Hash);
		assert_eq!(reason_for(HashKind::Crc), AutomaticMatchReasonEnum::CrcHash);
	}
}
