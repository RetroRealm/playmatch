use actix_web::HttpResponse;
use serde::{Deserialize, Serialize};
use utoipa::IntoParams;

pub const MAX_SEARCH_LITERAL_LEN: usize = 200;

/// Query for one entity by id.
#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct IdQuery {
	/// The IGDB id of the entity.
	pub id: i32,
}

/// Query for one entity by slug or id.
#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct SlugIdQuery {
	/// The IGDB slug of the entity.
	pub slug: Option<String>,
	/// The IGDB id of the entity.
	pub id: Option<i32>,
}

/// Query for multiple entities by their ids.
#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct IdsQuery {
	/// The IGDB ids of the entities.
	pub ids: Vec<i32>,
}

/// Query for a full-text entity search.
#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct SearchQuery {
	/// The search term.
	pub query: String,
}

pub fn validate_search_literal(value: &str) -> Result<(), HttpResponse> {
	let trimmed = value.trim();
	if trimmed.is_empty() {
		return Err(HttpResponse::BadRequest().body("search query must not be empty"));
	}
	if value.chars().count() > MAX_SEARCH_LITERAL_LEN {
		return Err(HttpResponse::BadRequest().body(format!(
			"search query must be at most {MAX_SEARCH_LITERAL_LEN} characters"
		)));
	}
	Ok(())
}
