use bigdecimal::BigDecimal;
use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
pub enum AgeRatingCategory {
	Esrb = 1,
	Pegi,
	Cero,
	Usk,
	Grac,
	Classind,
	Acb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
pub enum AgeRatingEnum {
	Three = 1,
	Seven,
	Twelve,
	Sixteen,
	Eighteen,
	RP,
	EC,
	E,
	E10,
	T,
	M,
	AO,
	Ceroa,
	Cerob,
	Ceroc,
	Cerod,
	Ceroz,
	USK0,
	USK6,
	USK12,
	USK16,
	USK18,
	GRACAll,
	GRAC12,
	GRAC15,
	GRAC18,
	GRACTesting,
	Gracindl,
	GRACIND10,
	GRACIND12,
	GRACIND14,
	GRACIND16,
	GRACIND18,
	Acbg,
	Acbpg,
	Acbm,
	ACBMA15,
	ACBR18,
	Acbrc,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgeRating {
	pub id: i32,
	pub category: AgeRatingCategory,
	pub checksum: Uuid,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub content_descriptions: Option<Vec<i64>>,
	pub rating: AgeRatingEnum,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub rating_cover_url: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub synopsis: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
pub enum AgeRatingContentCategory {
	EsrbAlcoholReference = 1,
	EsrbAnimatedBlood,
	EsrbBlood,
	EsrbBloodAndGore,
	EsrbCartoonViolence,
	EsrbComicMischief,
	EsrbCrudeHumor,
	EsrbDrugReference,
	EsrbFantasyViolence,
	EsrbIntenseViolence,
	EsrbLanguage,
	EsrbLyrics,
	EsrbMatureHumor,
	EsrbNudity,
	EsrbPartialNudity,
	EsrbRealGambling,
	EsrbSexualContent,
	EsrbSexualThemes,
	EsrbSexualViolence,
	EsrbSimulatedGambling,
	EsrbStrongLanguage,
	EsrbStrongLyrics,
	EsrbStrongSexualContent,
	EsrbSuggestiveThemes,
	EsrbTobaccoReference,
	EsrbUseOfAlcohol,
	EsrbUseOfDrugs,
	EsrbUseOfTobacco,
	EsrbViolence,
	EsrbViolentReferences,
	EsrbAnimatedViolence,
	EsrbMildLanguage,
	EsrbMildViolence,
	EsrbUseOfDrugsAndAlcohol,
	EsrbDrugAndAlcoholReference,
	EsrbMildSuggestiveThemes,
	EsrbMildCartoonViolence,
	EsrbMildBlood,
	EsrbRealisticBloodAndGore,
	EsrbRealisticViolence,
	EsrbAlcoholAndTobaccoReference,
	EsrbMatureSexualThemes,
	EsrbMildAnimatedViolence,
	EsrbMildSexualThemes,
	EsrbUseOfAlcoholAndTobacco,
	EsrbAnimatedBloodAndGore,
	EsrbMildFantasyViolence,
	EsrbMildLyrics,
	EsrbRealisticBlood,
	PegiViolence,
	PegiSex,
	PegiDrugs,
	PegiFear,
	PegiDiscrimination,
	PegiBadLanguage,
	PegiGambling,
	PegiOnlineGameplay,
	PegiInGamePurchases,
	CeroLove,
	CeroSexualContent,
	CeroViolence,
	CeroHorror,
	CeroDrinkingSmoking,
	CeroGambling,
	CeroCrime,
	CeroControlledSubstances,
	CeroLanguagesAndOthers,
	GracSexuality,
	GracViolence,
	GracFearHorrorThreatening,
	GracLanguage,
	GracAlcoholTobaccoDrug,
	GracCrimeAntiSocial,
	GracGambling,
	ClassIndViolencia,
	ClassIndViolenciaExtrema,
	ClassIndConteudoSexual,
	ClassIndNudez,
	ClassIndSexo,
	ClassIndSexoExplicito,
	ClassIndDrogas,
	ClassIndDrogasLicitas,
	ClassIndDrogasIlicitas,
	ClassIndLinguagemImpropria,
	ClassIndAtosCriminosos,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgeRatingContentDescription {
	pub id: i32,
	pub category: AgeRatingContentCategory,
	pub checksum: Uuid,
	pub description: String,
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
	pub checksum: Uuid,
	pub game: i32,
	pub height: i32,
	pub image_id: String,
	pub url: String,
	pub width: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
pub enum CharacterGender {
	Male = 0,
	Female = 1,
	Other = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
pub enum CharacterSpecies {
	Human = 1,
	Alien = 2,
	Animal = 3,
	Android = 4,
	Unknown = 5,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Character {
	pub id: i32,
	pub akas: Vec<String>,
	pub checksum: Uuid,
	pub country_name: String,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub description: String,
	pub games: Vec<i32>,
	pub gender: CharacterGender,
	pub mug_shot: i32,
	pub name: String,
	pub slug: String,
	pub species: CharacterSpecies,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
	pub url: String,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
pub enum CompanyChangeDateCategory {
	YYYYMMMMDD = 0,
	YYYYMMMM,
	YYYY,
	YYYYQ1,
	YYYYQ2,
	YYYYQ3,
	YYYYQ4,
	TBD,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
pub enum CompanyStartDateCategory {
	YYYYMMMMDD = 0,
	YYYYMMMM,
	YYYY,
	YYYYQ1,
	YYYYQ2,
	YYYYQ3,
	YYYYQ4,
	TBD,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Company {
	pub id: i32,
	pub change_date: i64,
	pub change_date_category: CompanyChangeDateCategory,
	pub changed_company_id: i32,
	pub checksum: Uuid,
	pub country: i32,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub description: String,
	pub developed: Vec<i32>,
	pub logo: i32,
	pub name: String,
	pub parent: i32,
	pub published: Vec<i32>,
	pub slug: String,
	pub start_date: i64,
	pub start_date_category: CompanyStartDateCategory,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
	pub url: String,
	pub websites: Vec<i32>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
pub enum CompanyWebsiteCategory {
	Official = 1,
	Wikia,
	Wikipedia,
	Facebook,
	Twitter,
	Twitch,
	Instagram = 8,
	Youtube,
	Iphone,
	Ipad,
	Android,
	Steam,
	Reddit,
	Itch,
	EpicGames,
	Gog,
	Discord,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CompanyWebsite {
	pub id: i32,
	pub category: CompanyWebsiteCategory,
	pub checksum: Uuid,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub trusted: Option<bool>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
pub enum ExternalGameCategory {
	Steam = 1,
	Gog = 5,
	Youtube = 10,
	Microsoft = 11,
	Apple = 13,
	Twitch = 14,
	Android = 15,
	AmazonAsin = 20,
	AmazonLuna = 22,
	AmazonAdg = 23,
	EpicGameStore = 26,
	Oculus = 28,
	Utomik = 29,
	ItchIo = 30,
	XboxMarketplace = 31,
	Kartridge = 32,
	PlaystationStoreUs = 36,
	FocusEntertainment = 37,
	XboxGamePassUltimateCloud = 54,
	Gamejolt = 55,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
pub enum ExternalGameMedia {
	Digital = 1,
	Physical = 2,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExternalGame {
	pub id: i32,
	pub category: ExternalGameCategory,
	pub checksum: Uuid,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub countries: Option<Vec<i32>>,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub game: i32,
	pub media: ExternalGameMedia,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
pub enum GameCategory {
	MainGame = 0,
	DlcAddon,
	Expansion,
	Bundle,
	StandaloneExpansion,
	Mod,
	Episode,
	Season,
	Remake,
	Remaster,
	ExpandedGame,
	Port,
	Fork,
	Pack,
	Update,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
pub enum GameStatus {
	Released = 0,
	Alpha = 2,
	Beta,
	EarlyAccess,
	Offline,
	Cancelled,
	Rumored,
	Delisted,
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
	pub category: GameCategory,
	pub checksum: Uuid,
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
	#[serde(skip_serializing_if = "Option::is_none")]
	pub status: Option<GameStatus>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
pub enum PlatformCategory {
	Console = 1,
	Arcade = 2,
	Platform = 3,
	OperatingSystem = 4,
	PortableConsole = 5,
	Computer = 6,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Platform {
	pub id: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub abbreviation: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub alternative_name: Option<String>,
	pub category: PlatformCategory,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
pub enum PlatformVersionReleaseDateCategory {
	YYYYMMMMDD = 0,
	YYYYMMMM = 1,
	YYYY = 2,
	YYYYQ1 = 3,
	YYYYQ2 = 4,
	YYYYQ3 = 5,
	YYYYQ4 = 6,
	TBD = 7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
pub enum PlatformVersionReleaseDateRegion {
	Europe = 1,
	NorthAmerica = 2,
	Australia = 3,
	NewZealand = 4,
	Japan = 5,
	China = 6,
	Asia = 7,
	Worldwide = 8,
	Korea = 9,
	Brazil = 10,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlatformVersionReleaseDate {
	pub id: i32,
	pub category: PlatformVersionReleaseDateCategory,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub date: i64,
	pub human: String,
	pub m: i32,
	pub platform_version: i32,
	pub region: PlatformVersionReleaseDateRegion,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
	pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
pub enum PlatformWebsiteCategory {
	Official = 1,
	Wikia = 2,
	Wikipedia = 3,
	Facebook = 4,
	Twitter = 5,
	Twitch = 6,
	Instagram = 8,
	YouTube = 9,
	IPhone = 10,
	IPad = 11,
	Android = 12,
	Steam = 13,
	Reddit = 14,
	Discord = 15,
	GooglePlus = 16,
	Tumblr = 17,
	LinkedIn = 18,
	Pinterest = 19,
	SoundCloud = 20,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlatformWebsite {
	pub id: i32,
	pub category: PlatformWebsiteCategory,
	pub checksum: Uuid,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub trusted: Option<bool>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u16)]
pub enum PopularitySource {
	Igdb = 121,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PopularityPrimitive {
	pub id: i32,
	#[serde(with = "ts_seconds")]
	pub calculated_at: DateTime<Utc>,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub game_id: i32,
	pub popularity_source: PopularitySource,
	pub popularity_type: i32,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
	pub value: BigDecimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PopularityType {
	pub id: i32,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	pub name: String,
	pub popularity_source: PopularitySource,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
pub enum ReleaseDateCategory {
	YYYYMMMMDD = 0,
	YYYYMMMM = 1,
	YYYY = 2,
	YYYYQ1 = 3,
	YYYYQ2 = 4,
	YYYYQ3 = 5,
	YYYYQ4 = 6,
	TBD = 7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
pub enum ReleaseDateRegion {
	Europe = 1,
	NorthAmerica = 2,
	Australia = 3,
	NewZealand = 4,
	Japan = 5,
	China = 6,
	Asia = 7,
	Worldwide = 8,
	Korea = 9,
	Brazil = 10,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReleaseDate {
	pub id: i32,
	pub category: ReleaseDateCategory,
	pub checksum: Uuid,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	#[serde(with = "ts_seconds")]
	pub date: DateTime<Utc>,
	pub game: i32,
	pub human: String,
	pub m: i32,
	pub platform: i32,
	pub region: ReleaseDateRegion,
	pub status: i32,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
	pub y: i32,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
pub enum WebsiteCategory {
	Official = 1,
	Wikia = 2,
	Wikipedia = 3,
	Facebook = 4,
	Twitter = 5,
	Twitch = 6,
	Instagram = 8,
	YouTube = 9,
	IPhone = 10,
	IPad = 11,
	Android = 12,
	Steam = 13,
	Reddit = 14,
	Itch = 15,
	EpicGames = 16,
	Gog = 17,
	Discord = 18,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Website {
	pub id: i32,
	pub category: WebsiteCategory,
	pub checksum: Uuid,
	pub game: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub trusted: Option<bool>,
	pub url: String,
}
