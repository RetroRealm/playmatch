use crate::error;
use crate::model::pagination::PageParams;
use crate::routes::handle_auth_and_permissions;
use crate::routes::v2::{build_time_page, resolve_or_return, resolve_time_keyset_start};
use actix_web::web::{Data, Query};
use actix_web::{HttpRequest, Responder, get};
use entity::sea_orm_active_enums::UserPermissionsEnum;
use sea_orm::DatabaseConnection;
use service::matching::suggestions::get_suggestions_page;
use uuid::Uuid;

const SUGGESTION_FILTER_TAG: u64 = 0x7375_6767_6573_7401;

/// Lists pending suggestions, newest first.
///
/// Suggestions are ordered by creation time, newest first. Requires credentials of
/// at least Automation level.
#[utoipa::path(
	get,
	tag = "Suggestion",
	params(PageParams),
	security(
		("bearer_auth" = [])
	),
	responses(
		(status = 200, description = "One page of suggestions, newest first", body = PageOfSuggestion),
		(status = 400, description = "Malformed pagination cursor or filter-set mismatch", body = V2ErrorBody),
		(status = 401, description = "Missing or invalid credentials"),
		(status = 403, description = "Credentials below Automation level"),
	)
)]
#[get("/suggestion")]
pub async fn list_suggestions_v2(
	params: Query<PageParams>,
	db_conn: Data<DatabaseConnection>,
	req: HttpRequest,
) -> error::Result<impl Responder> {
	handle_auth_and_permissions(&UserPermissionsEnum::Automation, req, db_conn.clone()).await?;

	let params = params.into_inner();
	let limit = params.limit_clamped();
	let start = resolve_or_return!(resolve_time_keyset_start(&params, SUGGESTION_FILTER_TAG));
	let has_previous = start.has_previous();

	let page = get_suggestions_page(start.after(), Some(limit), db_conn.get_ref()).await?;

	let positions: Vec<_> = page
		.rows
		.iter()
		.map(|s| (s.created_at, s.id))
		.collect::<Vec<(_, Uuid)>>();

	Ok(build_time_page(
		page.rows,
		&positions,
		page.has_more,
		has_previous,
		limit,
		SUGGESTION_FILTER_TAG,
	))
}
