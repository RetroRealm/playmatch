use crate::db::abstraction::ColumnEqIgnoreCaseTrait;
use entity::platform::ActiveModel;
use entity::prelude::Platform;
use entity::sea_orm_active_enums::{MatchTypeEnum, MetadataProviderEnum};
use entity::{company, dat_file, dat_file_import, game, platform, signature_metadata_mapping};
use sea_orm::ActiveValue::Set;
use sea_orm::prelude::Uuid;
use sea_orm::{
	ActiveModelTrait, ColumnTrait, DbConn, DbErr, EntityTrait, JoinType, LoaderTrait, ModelTrait,
	QueryFilter, QueryOrder, QuerySelect, RelationTrait, TryIntoModel,
};

/// Load a platform by id together with its company (if any) and every signature metadata mapping attached to it.
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

/// Return every platform paired with its company (if any) and its signature metadata mappings.
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

/// Find a platform by case-insensitive name, creating one with that name and optional company if it does not exist.
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

/// Return up to `limit` platforms that have no IGDB metadata mapping yet (or one with match_type None).
/// Returns `Ok(None)` when there is nothing left to process.
pub async fn get_unmatched_platforms_with_limit(
	limit: u64,
	conn: &DbConn,
) -> anyhow::Result<Option<Vec<platform::Model>>> {
	let res = Platform::find()
		.left_join(signature_metadata_mapping::Entity)
		.filter(
			signature_metadata_mapping::Column::Id
				.is_null()
				.or(signature_metadata_mapping::Column::MatchType.eq(MatchTypeEnum::None)),
		)
		.order_by_asc(platform::Column::Id)
		.limit(limit)
		.all(conn)
		.await?;

	if res.is_empty() {
		Ok(None)
	} else {
		Ok(Some(res))
	}
}

/// Resolve the platform a given game is attached to, via dat file and dat file import.
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

/// Return the IGDB signature metadata mapping attached to a platform, if any.
pub async fn find_platform_related_signature_metadata_mapping(
	model: &platform::Model,
	conn: &DbConn,
) -> Result<Option<signature_metadata_mapping::Model>, DbErr> {
	model
		.find_related(signature_metadata_mapping::Entity)
		.filter(signature_metadata_mapping::Column::Provider.eq(MetadataProviderEnum::Igdb))
		.one(conn)
		.await
}

/// Find a platform by exact (case-sensitive) name.
pub async fn find_platform_by_name(
	name: &str,
	conn: &DbConn,
) -> Result<Option<platform::Model>, DbErr> {
	Platform::find()
		.filter(platform::Column::Name.eq(name))
		.one(conn)
		.await
}
