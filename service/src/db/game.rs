use crate::db::abstraction::ColumnEqIgnoreCaseTrait;
use crate::ingestion::parser::model;
use ::entity::{
	game, game::Entity as Game, game_file, game_file::Entity as GameFile,
	signature_metadata_mapping,
};
use chrono::{Duration, NaiveDateTime, Utc};
use entity::sea_orm_active_enums::{FailedMatchReasonEnum, MatchTypeEnum, MetadataProviderEnum};
use entity::{company, dat_file, dat_file_import, game_file_presence, platform, signature_group};
use futures_util::future::BoxFuture;
use sea_orm::prelude::Uuid;
use sea_orm::sea_query::{Alias, Expr};
use sea_orm::{
	ActiveEnum, ActiveModelTrait, ActiveValue::Set, ColumnTrait, DbConn, DbErr, EntityTrait,
	JoinType, ModelTrait, QueryFilter, QueryOrder, QuerySelect, RelationTrait, TryIntoModel,
	sea_query::SimpleExpr,
};

/// Load a game by its internal UUID.
pub async fn get_game_by_id(game_id: Uuid, conn: &DbConn) -> Result<Option<game::Model>, DbErr> {
	Game::find_by_id(game_id).one(conn).await
}

/// Load the dat file import, dat file, signature group, platform, optional company and game files
/// associated with the given game. Errors if any required relation is missing.
pub async fn find_all_relations_of_game(
	game: &game::Model,
	conn: &DbConn,
) -> Result<
	(
		dat_file_import::Model,
		dat_file::Model,
		signature_group::Model,
		platform::Model,
		Option<company::Model>,
		Vec<game_file::Model>,
	),
	DbErr,
> {
	let dat_file_import = game
		.find_related(dat_file_import::Entity)
		.one(conn)
		.await?
		.ok_or_else(|| DbErr::RecordNotFound("Dat file import not found".to_string()))?;

	let dat_file = dat_file_import
		.find_related(dat_file::Entity)
		.one(conn)
		.await?
		.ok_or_else(|| DbErr::RecordNotFound("Dat file not found".to_string()))?;

	let (signature_group, platform) = tokio::try_join!(
		async {
			dat_file
				.find_related(signature_group::Entity)
				.one(conn)
				.await?
				.ok_or_else(|| DbErr::RecordNotFound("Signature group not found".to_string()))
		},
		async {
			dat_file
				.find_related(platform::Entity)
				.one(conn)
				.await?
				.ok_or_else(|| DbErr::RecordNotFound("Platform not found".to_string()))
		},
	)?;

	let (company, game_files) = tokio::try_join!(
		platform.find_related(company::Entity).one(conn),
		game.find_related(game_file::Entity).all(conn),
	)?;

	Ok((
		dat_file_import,
		dat_file,
		signature_group,
		platform,
		company,
		game_files,
	))
}

/// The dat file imports this hash was observed in, newest first.
pub async fn get_game_file_presence_history(
	game_file_id: Uuid,
	conn: &DbConn,
) -> Result<Vec<dat_file_import::Model>, DbErr> {
	let rows = game_file_presence::Entity::find()
		.filter(game_file_presence::Column::GameFileId.eq(game_file_id))
		.find_also_related(dat_file_import::Entity)
		.all(conn)
		.await?;

	let mut imports: Vec<dat_file_import::Model> =
		rows.into_iter().filter_map(|(_, import)| import).collect();
	imports.sort_by_key(|import| std::cmp::Reverse(import.imported_at));
	Ok(imports)
}

/// Insert a new game parsed from a dat file under the given import id.
pub async fn insert_game(
	dat_file_import_id: Uuid,
	game: model::Game,
	conn: &DbConn,
) -> Result<game::Model, DbErr> {
	let game = game::ActiveModel {
		dat_file_import_id: Set(dat_file_import_id),
		signature_group_internal_id: Set(game.id),
		signature_group_internal_clone_of_id: Set(game.cloneofid),
		name: Set(game.name),
		description: Set(game.description),
		categories: Set(game.category),
		..Default::default()
	};

	game.save(conn).await?.try_into_model()
}

/// Look up a game by the dat-file-provider's internal id, scoped to a single dat file.
pub async fn find_game_by_signature_group_internal_id_and_dat_file_id(
	signature_group_internal_id: String,
	dat_file_id: Uuid,
	conn: &DbConn,
) -> Result<Option<game::Model>, DbErr> {
	Game::find()
		.filter(game::Column::SignatureGroupInternalId.eq(signature_group_internal_id))
		.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
		.filter(dat_file_import::Column::DatFileId.eq(dat_file_id))
		.one(conn)
		.await
}

/// Look up a game by exact name, scoped to a single dat file. A merged dat file
/// can hold a live and a retired same-name row, so prefer the current one with a
/// stable id tiebreaker to keep repeated imports matching the same game.
pub async fn find_game_by_name_and_dat_file_id(
	name: &str,
	dat_file_id: Uuid,
	conn: &DbConn,
) -> Result<Option<game::Model>, DbErr> {
	Game::find()
		.filter(game::Column::Name.eq(name))
		.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
		.filter(dat_file_import::Column::DatFileId.eq(dat_file_id))
		.order_by_desc(game::Column::IsCurrent)
		.order_by_asc(game::Column::Id)
		.one(conn)
		.await
}

/// Load a game by id together with its primary signature metadata mapping, if any.
pub async fn find_game_and_id_mapping_by_game_id(
	game_id: Uuid,
	conn: &DbConn,
) -> Result<Option<(game::Model, Option<signature_metadata_mapping::Model>)>, DbErr> {
	let game = Game::find()
		.filter(game::Column::Id.eq(game_id))
		.one(conn)
		.await?;

	let mapping = signature_metadata_mapping::Entity::find()
		.filter(signature_metadata_mapping::Column::GameId.eq(game_id))
		.one(conn)
		.await?;

	if let Some(game) = game {
		Ok(Some((game, mapping)))
	} else {
		Ok(None)
	}
}

/// Find a game via any game file whose MD5 matches (case-insensitive), together with its metadata mappings.
pub async fn find_game_and_id_mapping_by_md5(
	md5: &str,
	conn: &DbConn,
) -> Result<Option<(game::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	find_signature_metadata_mapping_if_exists_by_filter(
		game_file::Column::Md5.eq_ignore_case(md5),
		conn,
	)
	.await
}

/// Find a game via any game file whose SHA1 matches (case-insensitive), together with its metadata mappings.
pub async fn find_game_and_id_mapping_by_sha1(
	sha1: &str,
	conn: &DbConn,
) -> Result<Option<(game::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	find_signature_metadata_mapping_if_exists_by_filter(
		game_file::Column::Sha1.eq_ignore_case(sha1),
		conn,
	)
	.await
}

/// Find a game via any game file whose SHA256 matches (case-insensitive), together with its metadata mappings.
pub async fn find_game_and_id_mapping_by_sha256(
	sha256: &str,
	conn: &DbConn,
) -> Result<Option<(game::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	find_signature_metadata_mapping_if_exists_by_filter(
		game_file::Column::Sha256.eq_ignore_case(sha256),
		conn,
	)
	.await
}

/// Find a game via a game file whose name (case-insensitive) and exact size both match, together with its metadata mappings.
pub async fn find_game_and_id_mapping_by_name_and_size(
	name: &str,
	size: i64,
	conn: &DbConn,
) -> Result<Option<(game::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	find_signature_metadata_mapping_if_exists_by_filter(
		game_file::Column::FileName
			.eq_ignore_case(name)
			.and(game_file::Column::FileSizeInBytes.eq(size)),
		conn,
	)
	.await
}

/// Return every game whose name matches exactly and whose dat file targets the given platform.
pub async fn find_games_by_name_and_platform_id(
	name: &str,
	platform_id: Uuid,
	conn: &DbConn,
) -> Result<Vec<game::Model>, DbErr> {
	Game::find()
		.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
		.join(
			JoinType::InnerJoin,
			dat_file_import::Relation::DatFile.def(),
		)
		.join(JoinType::InnerJoin, dat_file::Relation::Platform.def())
		.filter(
			game::Column::Name
				.eq(name)
				.and(platform::Column::Id.eq(platform_id)),
		)
		.all(conn)
		.await
}

/// Find a game whose file name matches exactly; if none, fall back to an exact match on the game name itself.
pub async fn find_game_by_name_or_game_file_name(
	name: &str,
	conn: &DbConn,
) -> Result<Option<game::Model>, DbErr> {
	let game_file = GameFile::find()
		.filter(game_file::Column::FileName.eq(name))
		.find_also_related(Game)
		.one(conn)
		.await?;

	if let Some((_, Some(game))) = game_file {
		return Ok(Some(game));
	}

	let game = Game::find()
		.filter(game::Column::Name.eq(name))
		.one(conn)
		.await?;

	match game {
		None => Ok(None),
		Some(game) => Ok(Some(game)),
	}
}

async fn find_signature_metadata_mapping_if_exists_by_filter(
	input: SimpleExpr,
	conn: &DbConn,
) -> Result<Option<(game::Model, Vec<signature_metadata_mapping::Model>)>, DbErr> {
	// A hash can match several rows (for example after a No-Intro rename leaves
	// the old retired entry alongside the new current one). Prefer the current
	// row, then a reconciled one (non-null last_seen) so a never-reconciled
	// orphan cannot win, then a stable id tiebreaker for determinism.
	let game_file = GameFile::find()
		.filter(input)
		.order_by_desc(game_file::Column::IsCurrent)
		.order_by_desc(game_file::Column::LastSeenDatFileImportId.is_not_null())
		.order_by_asc(game_file::Column::Id)
		.find_also_related(Game)
		.one(conn)
		.await?;

	match game_file {
		Some((_, Some(game))) => {
			let signature_metadata_mappings = signature_metadata_mapping::Entity::find()
				.filter(signature_metadata_mapping::Column::GameId.eq(game.id))
				.all(conn)
				.await?;

			Ok(Some((game, signature_metadata_mappings)))
		}
		_ => Ok(None),
	}
}

/// Return every game that is a clone-of the given game.
pub async fn find_all_children_of_game(
	game: &game::Model,
	conn: &DbConn,
) -> Result<Vec<game::Model>, DbErr> {
	Game::find()
		.filter(game::Column::CloneOf.eq(game.id))
		.all(conn)
		.await
}

/// Return the parent game (the game this one is a clone-of), if any.
pub async fn find_game_parent(
	game: &game::Model,
	conn: &DbConn,
) -> Result<Option<game::Model>, DbErr> {
	match game.clone_of {
		Some(clone_of_id) => {
			Game::find()
				.filter(game::Column::Id.eq(clone_of_id))
				.one(conn)
				.await
		}
		None => Ok(None),
	}
}

/// Return the signature metadata mapping attached to a game, if any.
pub async fn find_game_signature_metadata_mapping(
	game: &game::Model,
	conn: &DbConn,
) -> Result<Option<signature_metadata_mapping::Model>, DbErr> {
	signature_metadata_mapping::Entity::find()
		.filter(signature_metadata_mapping::Column::GameId.eq(game.id))
		.one(conn)
		.await
}

/// Return every signature metadata mapping row attached to the given game,
/// one per matched provider.
pub async fn find_all_signature_metadata_mappings_for_game(
	game_id: Uuid,
	conn: &DbConn,
) -> Result<Vec<signature_metadata_mapping::Model>, DbErr> {
	signature_metadata_mapping::Entity::find()
		.filter(signature_metadata_mapping::Column::GameId.eq(game_id))
		.all(conn)
		.await
}

/// Resolve the dat file id this game was imported from via its dat file import record.
pub async fn get_dat_file_id_of_game(game: &game::Model, conn: &DbConn) -> Result<Uuid, DbErr> {
	let dat_file_import = dat_file_import::Entity::find()
		.filter(dat_file_import::Column::Id.eq(game.dat_file_import_id))
		.one(conn)
		.await?;

	match dat_file_import {
		Some(dat_file_import) => Ok(dat_file_import.dat_file_id),
		None => Err(DbErr::RecordNotFound(
			"Dat file import not found".to_string(),
		)),
	}
}
/// Return every game that knows its signature-group-internal clone-of id but has no resolved `clone_of` UUID yet.
pub async fn get_unpopulated_clone_of_games(
	dat_file_id: Uuid,
	conn: &DbConn,
) -> Result<Vec<game::Model>, DbErr> {
	Game::find()
		.filter(
			game::Column::SignatureGroupInternalCloneOfId
				.is_not_null()
				.and(game::Column::CloneOf.is_null()),
		)
		.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
		.filter(dat_file_import::Column::DatFileId.eq(dat_file_id))
		.order_by_asc(game::Column::Id)
		.all(conn)
		.await
}

/// Return up to `page_size` games without a successful mapping for `provider` that are not
/// clones of any other game. Returns `Ok(None)` when there is nothing left to process.
pub fn get_unmatched_games_without_clone_of_with_limit<'a>(
	provider: MetadataProviderEnum,
	page_size: u64,
	cursor: Option<Uuid>,
	conn: DbConn,
) -> BoxFuture<'a, anyhow::Result<Option<Vec<game::Model>>>> {
	get_unmatched_games_with_limit(provider, true, true, page_size, cursor, conn)
}

/// Return up to `page_size` games without a successful mapping for `provider` that are
/// clones of another game. Returns `Ok(None)` when there is nothing left to process.
pub fn get_unmatched_games_with_clone_of_with_limit<'a>(
	provider: MetadataProviderEnum,
	page_size: u64,
	cursor: Option<Uuid>,
	conn: DbConn,
) -> BoxFuture<'a, anyhow::Result<Option<Vec<game::Model>>>> {
	get_unmatched_games_with_limit(provider, false, true, page_size, cursor, conn)
}

/// Same as [`get_unmatched_games_without_clone_of_with_limit`] but does not require the
/// game's platform to have a successful mapping for `provider`. Used by providers that
/// do not run a platform-matching pipeline (for example SteamGridDB, which searches by
/// game name without needing platform context).
pub fn get_unmatched_games_without_clone_of_with_limit_no_platform_gate<'a>(
	provider: MetadataProviderEnum,
	page_size: u64,
	cursor: Option<Uuid>,
	conn: DbConn,
) -> BoxFuture<'a, anyhow::Result<Option<Vec<game::Model>>>> {
	get_unmatched_games_with_limit(provider, true, false, page_size, cursor, conn)
}

/// Same as [`get_unmatched_games_with_clone_of_with_limit`] but without the platform
/// mapping gate. See [`get_unmatched_games_without_clone_of_with_limit_no_platform_gate`].
pub fn get_unmatched_games_with_clone_of_with_limit_no_platform_gate<'a>(
	provider: MetadataProviderEnum,
	page_size: u64,
	cursor: Option<Uuid>,
	conn: DbConn,
) -> BoxFuture<'a, anyhow::Result<Option<Vec<game::Model>>>> {
	get_unmatched_games_with_limit(provider, false, false, page_size, cursor, conn)
}

/// Min interval between cross-pass attempts on the same failed mapping.
/// Cross-pass results only change when sibling `matched_name` values churn,
/// which is typically a slow-moving signal. Re-running every cycle was the
/// CPU hot spot we are paying for.
pub const CROSS_MATCH_RETRY_INTERVAL_DAYS: i64 = 7;

/// Return up to `page_size` games where this `provider` is currently `Failed`
/// AND at least one sibling provider has matched the same game with a non-
/// null `matched_name` AND we have not cross-matched this row in the last
/// [`CROSS_MATCH_RETRY_INTERVAL_DAYS`]. Uses `EXISTS` (not a self-join) on
/// the sibling side so a game with multiple matched siblings appears once.
/// The new partial index `idx_smm_cross_match_pending` covers the outer
/// filter; `idx_smm_sibling_matched_name` covers the EXISTS subquery.
/// Returns `Ok(None)` when there is nothing left to process.
pub fn get_failed_games_for_cross_pass_with_limit<'a>(
	provider: MetadataProviderEnum,
	page_size: u64,
	cursor: Option<Uuid>,
	conn: DbConn,
) -> BoxFuture<'a, anyhow::Result<Option<Vec<game::Model>>>> {
	Box::pin(async move {
		let cooldown = Utc::now() - Duration::days(CROSS_MATCH_RETRY_INTERVAL_DAYS);
		let cooldown_naive: NaiveDateTime = cooldown.naive_utc();

		let mut query = Game::find()
			.join(
				JoinType::InnerJoin,
				game::Relation::SignatureMetadataMapping.def(),
			)
			.filter(
				signature_metadata_mapping::Column::Provider
					.eq(provider)
					.and(signature_metadata_mapping::Column::MatchType.eq(MatchTypeEnum::Failed))
					.and(
						signature_metadata_mapping::Column::FailedMatchReason
							.eq(FailedMatchReasonEnum::NoDirectMatch),
					)
					.and(
						signature_metadata_mapping::Column::CrossMatchLastTriedAt
							.is_null()
							.or(signature_metadata_mapping::Column::CrossMatchLastTriedAt
								.lt(cooldown_naive)),
					)
					.and(Expr::exists(
						sea_orm::sea_query::Query::select()
							.expr(Expr::val(1))
							.from(signature_metadata_mapping::Entity)
							.and_where(
								Expr::col(signature_metadata_mapping::Column::GameId)
									.equals((game::Entity, game::Column::Id)),
							)
							.and_where(
								Expr::col(signature_metadata_mapping::Column::Provider)
									.ne(provider.as_enum()),
							)
							.and_where(
								Expr::col(signature_metadata_mapping::Column::MatchType).is_in([
									MatchTypeEnum::Automatic.as_enum(),
									MatchTypeEnum::Manual.as_enum(),
								]),
							)
							.and_where(
								Expr::col(signature_metadata_mapping::Column::MatchedName)
									.is_not_null(),
							)
							.to_owned(),
					)),
			);

		if let Some(after) = cursor {
			query = query.filter(game::Column::Id.gt(after));
		}

		let res = query
			.order_by_asc(game::Column::Id)
			.limit(page_size)
			.all(&conn)
			.await?;

		if res.is_empty() {
			Ok(None)
		} else {
			Ok(Some(res))
		}
	})
}

/// Return up to `page_size` games whose last automatic match for `provider` failed with
/// `NoDirectMatch` more than 60 days ago. Used to retry stale failures.
/// Returns `Ok(None)` when there is nothing left to process.
pub fn get_automatic_match_failed_games_with_limit<'a>(
	provider: MetadataProviderEnum,
	page_size: u64,
	cursor: Option<Uuid>,
	conn: DbConn,
) -> BoxFuture<'a, anyhow::Result<Option<Vec<game::Model>>>> {
	Box::pin(async move {
		let sixty_days_ago = Utc::now() - Duration::days(60);
		let sixty_days_ago_naive: NaiveDateTime = sixty_days_ago.naive_utc();

		let mut query = Game::find()
			.join(
				JoinType::LeftJoin,
				game::Relation::SignatureMetadataMapping.def(),
			)
			.filter(
				signature_metadata_mapping::Column::MatchType
					.eq(MatchTypeEnum::Failed)
					.and(
						signature_metadata_mapping::Column::FailedMatchReason
							.eq(FailedMatchReasonEnum::NoDirectMatch),
					)
					.and(signature_metadata_mapping::Column::UpdatedAt.lt(sixty_days_ago_naive))
					.and(signature_metadata_mapping::Column::Provider.eq(provider)),
			);

		if let Some(after) = cursor {
			query = query.filter(game::Column::Id.gt(after));
		}

		let res = query
			.order_by_asc(game::Column::Id)
			.limit(page_size)
			.all(&conn)
			.await?;

		if res.is_empty() {
			Ok(None)
		} else {
			Ok(Some(res))
		}
	})
}

/// True if the provider has any game still needing a primary match: either an
/// unmatched game (no SMM row for this provider) or a stale
/// `Failed/NoDirectMatch` row past the 60-day retry window.
pub async fn has_outstanding_match_work_for_provider(
	provider: MetadataProviderEnum,
	conn: &DbConn,
) -> anyhow::Result<bool> {
	let unmatched_fut = async {
		let one = Game::find()
			.select_only()
			.column(game::Column::Id)
			.filter(
				Expr::exists(
					::sea_orm::sea_query::Query::select()
						.expr(Expr::val(1))
						.from(signature_metadata_mapping::Entity)
						.and_where(
							Expr::col(signature_metadata_mapping::Column::GameId)
								.equals((game::Entity, game::Column::Id)),
						)
						.and_where(
							Expr::col(signature_metadata_mapping::Column::Provider)
								.eq(provider.as_enum()),
						)
						.and_where(
							Expr::col(signature_metadata_mapping::Column::MatchType)
								.ne(MatchTypeEnum::None.as_enum()),
						)
						.to_owned(),
				)
				.not(),
			)
			.limit(1)
			.into_tuple::<Uuid>()
			.one(conn)
			.await?;
		anyhow::Ok(one.is_some())
	};

	let stale_failed_fut = async {
		let sixty_days_ago = Utc::now() - Duration::days(60);
		let sixty_days_ago_naive: NaiveDateTime = sixty_days_ago.naive_utc();
		let one = Game::find()
			.select_only()
			.column(game::Column::Id)
			.join(
				JoinType::InnerJoin,
				game::Relation::SignatureMetadataMapping.def(),
			)
			.filter(
				signature_metadata_mapping::Column::MatchType
					.eq(MatchTypeEnum::Failed)
					.and(
						signature_metadata_mapping::Column::FailedMatchReason
							.eq(FailedMatchReasonEnum::NoDirectMatch),
					)
					.and(signature_metadata_mapping::Column::UpdatedAt.lt(sixty_days_ago_naive))
					.and(signature_metadata_mapping::Column::Provider.eq(provider)),
			)
			.limit(1)
			.into_tuple::<Uuid>()
			.one(conn)
			.await?;
		anyhow::Ok(one.is_some())
	};

	let (unmatched, stale) = tokio::try_join!(unmatched_fut, stale_failed_fut)?;
	Ok(unmatched || stale)
}

/// True if the provider has any `Failed/NoDirectMatch` row whose cross-match
/// cooldown has elapsed and which has at least one sibling provider mapping
/// carrying a non-null `matched_name`.
pub async fn has_outstanding_cross_match_work_for_provider(
	provider: MetadataProviderEnum,
	conn: &DbConn,
) -> anyhow::Result<bool> {
	let cooldown = Utc::now() - Duration::days(CROSS_MATCH_RETRY_INTERVAL_DAYS);
	let cooldown_naive: NaiveDateTime = cooldown.naive_utc();

	let one = Game::find()
		.select_only()
		.column(game::Column::Id)
		.join(
			JoinType::InnerJoin,
			game::Relation::SignatureMetadataMapping.def(),
		)
		.filter(
			signature_metadata_mapping::Column::Provider
				.eq(provider)
				.and(signature_metadata_mapping::Column::MatchType.eq(MatchTypeEnum::Failed))
				.and(
					signature_metadata_mapping::Column::FailedMatchReason
						.eq(FailedMatchReasonEnum::NoDirectMatch),
				)
				.and(
					signature_metadata_mapping::Column::CrossMatchLastTriedAt
						.is_null()
						.or(signature_metadata_mapping::Column::CrossMatchLastTriedAt
							.lt(cooldown_naive)),
				)
				.and(Expr::exists(
					sea_orm::sea_query::Query::select()
						.expr(Expr::val(1))
						.from(signature_metadata_mapping::Entity)
						.and_where(
							Expr::col(signature_metadata_mapping::Column::GameId)
								.equals((game::Entity, game::Column::Id)),
						)
						.and_where(
							Expr::col(signature_metadata_mapping::Column::Provider)
								.ne(provider.as_enum()),
						)
						.and_where(
							Expr::col(signature_metadata_mapping::Column::MatchType).is_in([
								MatchTypeEnum::Automatic.as_enum(),
								MatchTypeEnum::Manual.as_enum(),
							]),
						)
						.and_where(
							Expr::col(signature_metadata_mapping::Column::MatchedName)
								.is_not_null(),
						)
						.to_owned(),
				)),
		)
		.limit(1)
		.into_tuple::<Uuid>()
		.one(conn)
		.await?;

	Ok(one.is_some())
}

fn get_unmatched_games_with_limit<'a>(
	provider: MetadataProviderEnum,
	clone_of_null: bool,
	require_platform_mapping: bool,
	page_size: u64,
	cursor: Option<Uuid>,
	conn: DbConn,
) -> BoxFuture<'a, anyhow::Result<Option<Vec<game::Model>>>> {
	Box::pin(async move {
		let smm1 = Alias::new("smm1");

		let mut query = Game::find()
			.join(JoinType::InnerJoin, game::Relation::DatFileImport.def())
			.join(
				JoinType::InnerJoin,
				dat_file_import::Relation::DatFile.def(),
			)
			.join(JoinType::InnerJoin, dat_file::Relation::Platform.def());

		if require_platform_mapping {
			query = query
				.join_as(
					JoinType::InnerJoin,
					platform::Relation::SignatureMetadataMapping.def(),
					smm1.clone(),
				)
				.filter(
					Expr::col((smm1.clone(), signature_metadata_mapping::Column::MatchType)).is_in(
						vec![
							MatchTypeEnum::Automatic.as_enum(),
							MatchTypeEnum::Manual.as_enum(),
						],
					),
				)
				.filter(
					Expr::col((smm1.clone(), signature_metadata_mapping::Column::Provider))
						.eq(provider.as_enum()),
				);
		}

		query = query
			.filter(if clone_of_null {
				game::Column::CloneOf.is_null()
			} else {
				game::Column::CloneOf.is_not_null()
			})
			.filter(
				Expr::exists(
					::sea_orm::sea_query::Query::select()
						.expr(Expr::val(1))
						.from(signature_metadata_mapping::Entity)
						.and_where(
							Expr::col(signature_metadata_mapping::Column::GameId)
								.equals((game::Entity, game::Column::Id)),
						)
						.and_where(
							Expr::col(signature_metadata_mapping::Column::Provider)
								.eq(provider.as_enum()),
						)
						.and_where(
							Expr::col(signature_metadata_mapping::Column::MatchType)
								.ne(MatchTypeEnum::None.as_enum()),
						)
						.to_owned(),
				)
				.not(),
			);

		if let Some(after) = cursor {
			query = query.filter(game::Column::Id.gt(after));
		}

		let res = query
			.order_by_asc(game::Column::Id)
			.limit(page_size)
			.all(&conn)
			.await?;

		if res.is_empty() {
			Ok(None)
		} else {
			Ok(Some(res))
		}
	})
}
