use actix_web::HttpResponse;
use serde::{Deserialize, Serialize};
use utoipa::IntoParams;

pub const MAX_SEARCH_LITERAL_LEN: usize = 200;

/// Query an entity by its ID
#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct IdQuery {
	pub id: i32,
}

/// Query an entity by its slug or ID
#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct SlugIdQuery {
	pub slug: Option<String>,
	pub id: Option<i32>,
}

/// Query multiple entities by their IDs
#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct IdsQuery {
	pub ids: Vec<i32>,
}

/// Query to search for entities by a string
#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct SearchQuery {
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
