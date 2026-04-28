mod abstraction;
pub mod company;
pub mod constants;
pub mod dat_file;
pub mod dat_file_import;
pub mod game;
pub mod game_file;
pub mod launchbox;
pub mod platform;
pub mod signature_group;
pub mod signature_metadata_mapping;
pub mod signature_metadata_mapping_suggestions;
pub mod user;

/// Generate a `get_unmatched_*_with_limit(provider, limit, conn)` fn that returns up to
/// `limit` rows of `$entity` with no signature metadata mapping for the given provider
/// (or one whose match_type is `None`). The join is provider-scoped via `on_condition`
/// so rows matched to a different provider still appear as unmatched for this one.
///
/// `$fk_column` is the signature_metadata_mapping column pointing back to `$entity`
/// (e.g. `signature_metadata_mapping::Column::CompanyId`).
///
/// Returns a `BoxFuture` and owns its `DbConn` so the fn is usable as a non-generic
/// function pointer in the shared matcher pipeline driver.
///
/// Expects `signature_metadata_mapping`, `MatchTypeEnum`, sea-orm query traits, and the
/// entity's prelude type (`$entity`) to be in scope at the call site.
macro_rules! unmatched_entities_with_limit {
	($(#[$meta:meta])* $fn:ident, $entity:ident, $model:ty, $column:expr, $fk_column:expr) => {
		$(#[$meta])*
		pub fn $fn(
			provider: ::entity::sea_orm_active_enums::MetadataProviderEnum,
			limit: u64,
			conn: ::sea_orm::DbConn,
		) -> ::futures_util::future::BoxFuture<'static, ::anyhow::Result<Option<Vec<$model>>>> {
			Box::pin(async move {
				use ::sea_orm::ActiveEnum;
				let rows = $entity::find()
					.filter(
						::sea_orm::sea_query::Expr::exists(
							::sea_orm::sea_query::Query::select()
								.expr(::sea_orm::sea_query::Expr::val(1))
								.from(signature_metadata_mapping::Entity)
								.and_where(
									::sea_orm::sea_query::Expr::col($fk_column)
										.equals(($entity, $column)),
								)
								.and_where(
									::sea_orm::sea_query::Expr::col(
										signature_metadata_mapping::Column::Provider,
									)
									.eq(provider.as_enum()),
								)
								.and_where(
									::sea_orm::sea_query::Expr::col(
										signature_metadata_mapping::Column::MatchType,
									)
									.ne(MatchTypeEnum::None.as_enum()),
								)
								.to_owned(),
						)
						.not(),
					)
					.order_by_asc($column)
					.limit(limit)
					.all(&conn)
					.await?;

				if rows.is_empty() {
					Ok(None)
				} else {
					Ok(Some(rows))
				}
			})
		}
	};
}
pub(crate) use unmatched_entities_with_limit;
