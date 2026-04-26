//! Provider-agnostic route wrappers. Each provider defines a thin prefilled
//! wrapper macro in its own `routes/<provider>.rs` so per-entity invocations
//! stay terse.
//!
//! The macros assume the following are in scope at the invocation site:
//! `actix_web::web::Data`, `actix_web::{HttpResponse, Responder, get}`,
//! `actix_web_lab::extract::Query`, `redis::aio::MultiplexedConnection`,
//! `crate::error`, `crate::model::igdb::{IdQuery, IdsQuery}`, and
//! `crate::util::igdb_route_mutli_id_helper`.

/// Single-id proxy route: `GET $route?id=N` cached through `$cached_fn`.
#[macro_export]
macro_rules! __provider_id_route_impl {
	(
		$client_ty:ty,
		$tag:literal,
		$route:literal,
		$fn_name:ident,
		$cached_fn:ident,
		$model:ty
	) => {
		#[utoipa::path(
			get,
			context_path = "/api",
			tag = $tag,
			params(IdQuery),
			responses(
				(status = 200, description = "Returns provider metadata for the requested id", body = $model),
				(status = 404, description = "Not found")
			)
		)]
		#[get($route)]
		pub async fn $fn_name(
			query: Query<IdQuery>,
			redis_conn: Data<MultiplexedConnection>,
			provider_client: Data<$client_ty>,
		) -> error::Result<impl Responder> {
			let response = $cached_fn(
				provider_client.as_ref(),
				&mut redis_conn.get_ref().clone(),
				query.into_inner().id,
			)
			.await?;

			if response.is_none() {
				return Ok(HttpResponse::NotFound().finish());
			}

			Ok(HttpResponse::Ok().json(response))
		}
	};
}

/// Bulk-ids proxy route: `GET $route?ids=N,N,...` cached per id through `$cached_fn`,
/// fanned out in parallel via `igdb_route_mutli_id_helper`.
#[macro_export]
macro_rules! __provider_ids_route_impl {
	(
		$client_ty:ty,
		$tag:literal,
		$route:literal,
		$fn_name:ident,
		$cached_fn:ident,
		$model:ty
	) => {
		#[utoipa::path(
			get,
			context_path = "/api",
			tag = $tag,
			params(IdsQuery),
			responses(
				(status = 200, description = "Returns provider metadata for the requested ids", body = Vec<$model>)
			)
		)]
		#[get($route)]
		pub async fn $fn_name(
			query: Query<IdsQuery>,
			redis_conn: Data<MultiplexedConnection>,
			provider_client: Data<$client_ty>,
		) -> error::Result<impl Responder> {
			let redis_conn = redis_conn.get_ref().clone();

			let response = igdb_route_mutli_id_helper::<$model>(query.into_inner().ids, |id| {
				tokio::spawn({
					let client = provider_client.clone();
					let mut redis_conn = redis_conn.clone();
					async move { $cached_fn(client.as_ref(), &mut redis_conn, id).await }
				})
			})
			.await?;

			Ok(HttpResponse::Ok().json(response))
		}
	};
}

/// Pastes the singular-id and bulk-ids route impls together. Kept as two impls
/// because utoipa cannot toggle `body = $model` vs `body = Vec<$model>` inside
/// a single derive input. Pluralisation is irregular, so both fn names and
/// both routes are passed explicitly.
#[macro_export]
macro_rules! __provider_entity_routes_impl {
	(
		$client_ty:ty,
		$tag:literal,
		$singular_route:literal,
		$singular_fn:ident,
		$plural_route:literal,
		$plural_fn:ident,
		$cached_fn:ident,
		$model:ty
	) => {
		$crate::__provider_id_route_impl!(
			$client_ty,
			$tag,
			$singular_route,
			$singular_fn,
			$cached_fn,
			$model
		);
		$crate::__provider_ids_route_impl!(
			$client_ty,
			$tag,
			$plural_route,
			$plural_fn,
			$cached_fn,
			$model
		);
	};
}
