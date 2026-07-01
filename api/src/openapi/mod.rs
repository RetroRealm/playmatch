mod v1;
mod v2;

pub use v1::ApiDocV1;
pub use v2::ApiDocV2;

use utoipa::OpenApi;
use utoipa::openapi::ComponentsBuilder;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};

/// Single, non-bypassable constructor that finalizes any version's document by
/// folding in the bearer security scheme. Both per-version builders route
/// through this so the auth scheme can never be forgotten on one of them.
fn build_openapi(mut openapi: utoipa::openapi::OpenApi) -> utoipa::openapi::OpenApi {
	let existing_components = openapi.components.take().unwrap_or_default();

	let new_components = ComponentsBuilder::from(existing_components)
		.security_scheme(
			"bearer_auth",
			SecurityScheme::Http(
				HttpBuilder::new()
					.scheme(HttpAuthScheme::Bearer)
					.bearer_format("JWT")
					.build(),
			),
		)
		.build();

	openapi.components = Some(new_components);
	openapi
}

pub fn create_openapi_v1() -> utoipa::openapi::OpenApi {
	build_openapi(ApiDocV1::openapi())
}

pub fn create_openapi_v2() -> utoipa::openapi::OpenApi {
	build_openapi(ApiDocV2::openapi())
}

/// Transitional alias retained so existing callers and the frozen v1 surface
/// test keep resolving to the v1 document.
pub fn create_openapi() -> utoipa::openapi::OpenApi {
	create_openapi_v1()
}
