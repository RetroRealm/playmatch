mod abstraction;
pub mod company;
pub mod constants;
pub mod dat_file;
pub mod dat_file_import;
pub mod game;
pub mod game_file;
pub mod platform;
pub mod signature_group;
pub mod signature_metadata_mapping;
pub mod signature_metadata_mapping_suggestions;
pub mod user;

/// Generate a `get_unmatched_*_with_limit` fn that returns up to `limit` rows of `$entity`
/// that have no IGDB signature metadata mapping (or one with match_type `None`).
///
/// Expects `signature_metadata_mapping`, `MatchTypeEnum`, sea-orm query traits, and the
/// entity's prelude type (`$entity`) to be in scope at the call site.
macro_rules! unmatched_entities_with_limit {
	($(#[$meta:meta])* $fn:ident, $entity:ident, $model:ty, $column:expr) => {
		$(#[$meta])*
		pub async fn $fn(
			limit: u64,
			conn: &::sea_orm::DbConn,
		) -> ::anyhow::Result<Option<Vec<$model>>> {
			let rows = $entity::find()
				.left_join(signature_metadata_mapping::Entity)
				.filter(
					signature_metadata_mapping::Column::Id
						.is_null()
						.or(signature_metadata_mapping::Column::MatchType.eq(MatchTypeEnum::None)),
				)
				.order_by_asc($column)
				.limit(limit)
				.all(conn)
				.await?;

			if rows.is_empty() {
				Ok(None)
			} else {
				Ok(Some(rows))
			}
		}
	};
}
pub(crate) use unmatched_entities_with_limit;
