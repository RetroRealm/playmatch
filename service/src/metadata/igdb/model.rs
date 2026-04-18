use bigdecimal::BigDecimal;
use chrono::serde::{ts_seconds, ts_seconds_option};
use chrono::{DateTime, Utc};
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgeRating {
	pub id: i32,
	/// Deprecated by IGDB. Use `organization` instead.
	#[schema(deprecated)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub category: Option<i32>,
	pub checksum: Uuid,
	/// Deprecated by IGDB. Use `rating_content_descriptions` instead.
	#[schema(deprecated)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub content_descriptions: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub organization: Option<i32>,
	/// Deprecated by IGDB. Use `rating_category` instead.
	#[schema(deprecated)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub rating: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub rating_category: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub rating_content_descriptions: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub rating_cover_url: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub synopsis: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgeRatingCategory {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub organization: i32,
	pub rating: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgeRatingContentDescriptionV2 {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub description: String,
	pub description_type: i32,
	pub organization: i32,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgeRatingContentDescriptionType {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub name: String,
	pub slug: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgeRatingOrganization {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub name: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AlternativeName {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub comment: Option<String>,
	pub game: i32,
	pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Artwork {
	pub id: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub alpha_channel: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub animated: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub artwork_type: Option<i32>,
	pub checksum: Uuid,
	pub game: i32,
	pub height: i32,
	pub image_id: String,
	pub url: String,
	pub width: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArtworkType {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub name: String,
	pub slug: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Character {
	pub id: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub akas: Option<Vec<String>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub character_gender: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub character_species: Option<i32>,
	pub checksum: Uuid,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub country_name: Option<String>,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub games: Option<Vec<i32>>,
	/// Deprecated by IGDB. Use `character_gender` instead.
	#[schema(deprecated)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub gender: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub mug_shot: Option<i32>,
	pub name: String,
	pub slug: String,
	/// Deprecated by IGDB. Use `character_species` instead.
	#[schema(deprecated)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub species: Option<i32>,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
	pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CharacterMugShot {
	pub id: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub alpha_channel: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub animated: Option<bool>,
	pub checksum: Uuid,
	pub height: i32,
	pub image_id: String,
	pub url: String,
	pub width: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CharacterGender {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub name: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CharacterSpecies {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub name: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Collection {
	pub id: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub as_child_relations: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub as_parent_relations: Option<Vec<i32>>,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub games: Vec<i32>,
	pub name: String,
	pub slug: String,
	#[serde(rename = "type")]
	pub r#type: i32,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
	pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CollectionMembership {
	pub id: i32,
	pub checksum: Uuid,
	pub collection: i32,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub game: i32,
	pub r#type: i32,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CollectionMembershipType {
	pub id: i32,
	pub allowed_collection_type: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub description: String,
	pub name: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CollectionRelation {
	pub id: i32,
	pub checksum: Uuid,
	pub child_collection: i32,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub parent_collection: i32,
	pub r#type: i32,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CollectionRelationType {
	pub id: i32,
	pub allowed_child_type: i32,
	pub allowed_parent_type: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub description: String,
	pub name: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CollectionType {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub description: String,
	pub name: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Company {
	pub id: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub change_date: Option<i64>,
	/// Deprecated by IGDB. Use `change_date_format` instead.
	#[schema(deprecated)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub change_date_category: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub change_date_format: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub changed_company_id: Option<i32>,
	pub checksum: Uuid,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub company_size: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub company_type_histories: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub country: Option<i32>,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub developed: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub logo: Option<i32>,
	pub name: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub parent: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub published: Option<Vec<i32>>,
	pub slug: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub start_date: Option<i64>,
	/// Deprecated by IGDB. Use `start_date_format` instead.
	#[schema(deprecated)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub start_date_category: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub start_date_format: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub status: Option<i32>,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub url: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub websites: Option<Vec<i32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CompanySize {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub name: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CompanyLogo {
	pub id: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub alpha_channel: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub animated: Option<bool>,
	pub checksum: Uuid,
	pub height: i32,
	pub image_id: String,
	pub url: String,
	pub width: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CompanyStatus {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub name: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CompanyType {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub name: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CompanyTypeHistory {
	pub id: i32,
	pub checksum: Uuid,
	pub company: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub company_type: Option<i32>,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub parent_company: Option<i32>,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CompanyWebsite {
	pub id: i32,
	/// Deprecated by IGDB. Use `type` instead.
	#[schema(deprecated)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub category: Option<i32>,
	pub checksum: Uuid,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub trusted: Option<bool>,
	#[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
	pub r#type: Option<i32>,
	pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Cover {
	pub id: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub alpha_channel: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub animated: Option<bool>,
	pub checksum: Uuid,
	pub game: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub game_localization: Option<i32>,
	pub height: i32,
	pub image_id: String,
	pub url: String,
	pub width: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DateFormat {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub format: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EntityType {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
	pub name: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Event {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub description: String,
	#[serde(with = "ts_seconds")]
	pub end_time: DateTime<Utc>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub event_logo: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub event_networks: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub games: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub live_stream_url: Option<String>,
	pub name: String,
	pub slug: String,
	#[serde(with = "ts_seconds")]
	pub start_time: DateTime<Utc>,
	pub time_zone: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub videos: Option<Vec<i32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EventLogo {
	pub id: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub alpha_channel: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub animated: Option<bool>,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub event: i32,
	pub height: i32,
	pub image_id: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
	pub url: String,
	pub width: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EventNetwork {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub event: i32,
	pub network_type: i32,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
	pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExternalGame {
	pub id: i32,
	/// Deprecated by IGDB. Use `external_game_source` instead.
	#[schema(deprecated)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub category: Option<i32>,
	pub checksum: Uuid,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub countries: Option<Vec<i32>>,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub external_game_source: Option<i32>,
	pub game: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub game_release_format: Option<i32>,
	/// Deprecated by IGDB. Use `game_release_format` instead.
	#[schema(deprecated)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub media: Option<i32>,
	pub name: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub platform: Option<i32>,
	pub uid: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
	pub url: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub year: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExternalGameSource {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub name: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Franchise {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub games: Option<Vec<i32>>,
	pub name: String,
	pub slug: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
	pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Game {
	pub id: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub age_ratings: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub aggregated_rating: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub aggregated_rating_count: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub alternative_names: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub artworks: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub bundles: Option<Vec<i32>>,
	/// Deprecated by IGDB. Use `game_type` instead.
	#[schema(deprecated)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub category: Option<i32>,
	pub checksum: Uuid,
	/// Deprecated by IGDB. Use `collections` instead.
	#[schema(deprecated)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub collection: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub collections: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub cover: Option<i32>,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub dlcs: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub expanded_games: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub expansions: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub external_games: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub first_release_date: Option<i64>,
	/// Deprecated by IGDB. Scheduled for removal.
	#[schema(deprecated)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub follows: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub forks: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub franchise: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub franchises: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub game_engines: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub game_localizations: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub game_modes: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub game_status: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub game_type: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub genres: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub hypes: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub involved_companies: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub keywords: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub language_supports: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub multiplayer_modes: Option<Vec<i32>>,
	pub name: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub parent_game: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub platforms: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub player_perspectives: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ports: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub rating: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub rating_count: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub release_dates: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub remakes: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub remasters: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub screenshots: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub similar_games: Option<Vec<i32>>,
	pub slug: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub standalone_expansions: Option<Vec<i32>>,
	/// Deprecated by IGDB. Use `game_status` instead.
	#[schema(deprecated)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub status: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub storyline: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub summary: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tags: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub themes: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub total_rating: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub total_rating_count: Option<i32>,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
	pub url: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub version_parent: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub version_title: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub videos: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub websites: Option<Vec<i32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GameEngine {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub companies: Option<Vec<i32>>,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub logo: Option<i32>,
	pub name: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub platforms: Option<Vec<i32>>,
	pub slug: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
	pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GameEngineLogo {
	pub id: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub alpha_channel: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub animated: Option<bool>,
	pub checksum: Uuid,
	pub height: i32,
	pub image_id: String,
	pub url: String,
	pub width: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GameLocalization {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub cover: Option<i32>,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub game: i32,
	pub name: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub region: Option<i32>,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GameMode {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub name: String,
	pub slug: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
	pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GameReleaseFormat {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub format: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GameStatus {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub status: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GameTimeToBeat {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub completely: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub count: Option<i32>,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub game_id: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub hastily: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub normally: Option<i32>,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GameType {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	#[serde(rename = "type")]
	pub r#type: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GameVersion {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub features: Option<Vec<i32>>,
	pub game: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub games: Option<Vec<i32>>,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
	pub url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
pub enum GameVersionFeatureCategory {
	Boolean = 0,
	Description = 1,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GameVersionFeature {
	pub id: i32,
	pub category: GameVersionFeatureCategory,
	pub checksum: Uuid,
	pub description: String,
	pub position: i32,
	pub title: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub values: Option<Vec<i32>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
pub enum GameVersionFeatureValueEnum {
	NotIncluded = 0,
	Included = 1,
	PreOrderOnly = 2,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GameVersionFeatureValue {
	pub id: i32,
	pub checksum: Uuid,
	pub game: i32,
	pub game_feature: i32,
	pub included_feature: GameVersionFeatureValueEnum,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GameVideo {
	pub id: i32,
	pub checksum: Uuid,
	pub game: i32,
	pub name: String,
	pub video_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Genre {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub name: String,
	pub slug: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
	pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InvolvedCompany {
	pub id: i32,
	pub checksum: Uuid,
	pub company: i32,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub developer: bool,
	pub game: i32,
	pub porting: bool,
	pub publisher: bool,
	pub supporting: bool,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Keyword {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub name: String,
	pub slug: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
	pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Language {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub locale: String,
	pub name: String,
	pub native_name: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LanguageSupport {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub game: i32,
	pub language: i32,
	pub language_support_type: i32,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LanguageSupportType {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub name: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MultiplayerMode {
	pub id: i32,
	pub campaigncoop: bool,
	pub checksum: Uuid,
	pub dropin: bool,
	pub game: i32,
	pub lancoop: bool,
	pub offlinecoop: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub offlinecoopmax: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub offlinemax: Option<i32>,
	pub onlinecoop: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub onlinecoopmax: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub onlinemax: Option<i32>,
	pub platform: i32,
	pub splitscreen: bool,
	pub splitscreenonline: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NetworkType {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub event_networks: Option<Vec<i32>>,
	pub name: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Platform {
	pub id: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub abbreviation: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub alternative_name: Option<String>,
	/// Deprecated by IGDB. Use `platform_type` instead.
	#[schema(deprecated)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub category: Option<i32>,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub generation: Option<i32>,
	pub name: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub platform_family: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub platform_logo: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub platform_type: Option<i32>,
	pub slug: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub summary: Option<String>,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
	pub url: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub versions: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub websites: Option<Vec<i32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlatformFamily {
	pub id: i32,
	pub checksum: Uuid,
	pub name: String,
	pub slug: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlatformLogo {
	pub id: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub alpha_channel: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub animated: Option<bool>,
	pub checksum: Uuid,
	pub height: i32,
	pub image_id: String,
	pub url: String,
	pub width: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlatformType {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub name: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlatformVersion {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub companies: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub connectivity: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub cpu: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub graphics: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub main_manufacturer: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub media: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub memory: Option<String>,
	pub name: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub os: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub output: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub platform_logo: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub platform_version_release_dates: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub resolutions: Option<String>,
	pub slug: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub sound: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub storage: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub summary: Option<String>,
	pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlatformVersionCompany {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub comment: Option<String>,
	pub company: i32,
	pub developer: bool,
	pub manufacturer: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlatformVersionReleaseDate {
	pub id: i32,
	/// Deprecated by IGDB. Use `date_format` instead.
	#[schema(deprecated)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub category: Option<i32>,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub date: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub date_format: Option<i32>,
	pub human: String,
	pub m: i32,
	pub platform_version: i32,
	/// Deprecated by IGDB. Use `release_region` instead.
	#[schema(deprecated)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub region: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub release_region: Option<i32>,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
	pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlatformWebsite {
	pub id: i32,
	/// Deprecated by IGDB. Use `type` instead.
	#[schema(deprecated)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub category: Option<i32>,
	pub checksum: Uuid,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub trusted: Option<bool>,
	#[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
	pub r#type: Option<i32>,
	pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlayerPerspective {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub name: String,
	pub slug: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
	pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PopularityPrimitive {
	pub id: i32,
	#[serde(with = "ts_seconds")]
	pub calculated_at: DateTime<Utc>,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub external_popularity_source: Option<i32>,
	pub game_id: i32,
	/// Deprecated by IGDB. Use `external_popularity_source` instead.
	#[schema(deprecated)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub popularity_source: Option<i32>,
	pub popularity_type: i32,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
	#[schema(value_type = String)]
	pub value: BigDecimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PopularityType {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub external_popularity_source: Option<i32>,
	pub name: String,
	/// Deprecated by IGDB. Use `external_popularity_source` instead.
	#[schema(deprecated)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub popularity_source: Option<i32>,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Region {
	pub id: i32,
	pub category: String,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub identifier: String,
	pub name: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReleaseDate {
	pub id: i32,
	/// Deprecated by IGDB. Use `date_format` instead.
	#[schema(deprecated)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub category: Option<i32>,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub d: Option<i32>,
	#[serde(default, skip_serializing_if = "Option::is_none", with = "ts_seconds_option")]
	pub date: Option<DateTime<Utc>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub date_format: Option<i32>,
	pub game: i32,
	pub human: String,
	pub m: i32,
	pub platform: i32,
	/// Deprecated by IGDB. Use `release_region` instead.
	#[schema(deprecated)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub region: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub release_region: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub status: Option<i32>,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
	pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReleaseDateRegion {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub region: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReleaseDateStatus {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub description: String,
	pub name: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Report {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub entity_type: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub report_type: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub source_item_id: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub target_item_id: Option<i32>,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReportType {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub name: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Screenshot {
	pub id: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub alpha_channel: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub animated: Option<bool>,
	pub checksum: Uuid,
	pub game: i32,
	pub height: i32,
	pub image_id: String,
	pub url: String,
	pub width: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Theme {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub name: String,
	pub slug: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
	pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Website {
	pub id: i32,
	/// Deprecated by IGDB. Use `type` instead.
	#[schema(deprecated)]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub category: Option<i32>,
	pub checksum: Uuid,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub game: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub trusted: Option<bool>,
	#[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
	pub r#type: Option<i32>,
	pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WebsiteType {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	#[serde(rename = "type")]
	pub r#type: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
}
