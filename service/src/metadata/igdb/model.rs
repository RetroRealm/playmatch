use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
pub enum Category {
	Console = 1,
	Arcade,
	Platform,
	OperatingSystem,
	PortableConsole,
	Computer,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Platform {
	pub id: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub abbreviation: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub alternative_name: Option<String>,
	pub category: Category,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
pub enum ChangeDateCategory {
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
pub enum StartDateCategory {
	YYYYMMMMDD = 0,
	YYYYMMMM = 1,
	YYYY = 2,
	YYYYQ1 = 3,
	YYYYQ2 = 4,
	YYYYQ3 = 5,
	YYYYQ4 = 6,
	TBD = 7,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Company {
	pub id: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub change_date: Option<i64>,
	pub change_date_category: ChangeDateCategory,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub changed_company_id: Option<i32>,
	pub checksum: Uuid,
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
	#[serde(skip_serializing_if = "Option::is_none")]
	pub start_date_category: Option<StartDateCategory>,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
	pub url: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub websites: Option<Vec<i32>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
pub enum GameCategory {
	MainGame,
	DLCAddon,
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
	pub category: GameCategory,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub external_games: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub expanded_games: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub expansions: Option<Vec<i32>>,
	pub name: String,
	pub slug: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
	pub url: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub websites: Option<Vec<i32>>,
	pub checksum: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub cover: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub game_modes: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub genres: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub involved_companies: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub summary: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub language_supports: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub bundles: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub first_release_date: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub forks: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub platforms: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub player_perspectives: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub release_dates: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub similar_games: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub tags: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub themes: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub artworks: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub screenshots: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub age_ratings: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub aggregated_rating: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub aggregated_rating_count: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub total_rating: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub total_rating_count: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub status: Option<GameStatus>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub videos: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub parent_game: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub game_engines: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub keywords: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub storyline: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub rating: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub rating_count: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub alternative_names: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub collection: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub collections: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub game_localizations: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub version_parent: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub version_title: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub follows: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub franchises: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub dlcs: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub hypes: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub multiplayer_modes: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ports: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub remakes: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub remasters: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub standalone_expansions: Option<Vec<i32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Genre {
	checksum: Uuid,
	#[serde(with = "ts_seconds")]
	created_at: DateTime<Utc>,
	name: String,
	slug: String,
	#[serde(with = "ts_seconds")]
	updated_at: DateTime<Utc>,
	url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Franchise {
	checksum: Uuid,
	#[serde(with = "ts_seconds")]
	created_at: DateTime<Utc>,
	games: Vec<i32>,
	name: String,
	slug: String,
	#[serde(with = "ts_seconds")]
	updated_at: DateTime<Utc>,
	url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
pub enum ExternalGameCategory {
	Steam = 1,
	GoodOldGames = 5,
	Youtube = 10,
	Microsoft = 11,
	Apple = 13,
	Twitch = 14,
	Android = 15,
	AmazonAsin = 20,
	AmazonLuna = 22,
	AmazonAdg = 23,
	EpicGames = 26,
	Oculus = 28,
	Utomik = 29,
	ItchIo = 30,
	XboxMarketplace = 31,
	Kartridge = 32,
	PlayStationStoreUS = 36,
	FocusEntertainment = 37,
	XboxGamePassUltimateCloud = 54,
	Gamejolt = 55,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
pub enum ExternalGameMedia {
	Digital = 1,
	Physical,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExternalGame {
	category: ExternalGameCategory,
	checksum: Uuid,
	#[serde(skip_serializing_if = "Option::is_none")]
	countries: Option<Vec<i32>>,
	#[serde(with = "ts_seconds")]
	created_at: DateTime<Utc>,
	game: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	media: Option<ExternalGameMedia>,
	#[serde(skip_serializing_if = "Option::is_none")]
	name: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	platform: Option<i32>,
	uid: String,
	#[serde(with = "ts_seconds")]
	updated_at: DateTime<Utc>,
	url: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	year: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Cover {
	#[serde(skip_serializing_if = "Option::is_none")]
	alpha_channel: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	animated: Option<bool>,
	checksum: Uuid,
	game: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	game_localization: Option<i32>,
	height: i32,
	image_id: String,
	url: String,
	width: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Collection {
	#[serde(skip_serializing_if = "Option::is_none")]
	as_child_relations: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	as_parent_relations: Option<Vec<i32>>,
	checksum: Uuid,
	#[serde(with = "ts_seconds")]
	created_at: DateTime<Utc>,
	games: Vec<i32>,
	name: String,
	slug: String,
	#[serde(rename = "type")]
	r#type: i32,
	#[serde(with = "ts_seconds")]
	updated_at: DateTime<Utc>,
	url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Artwork {
	#[serde(skip_serializing_if = "Option::is_none")]
	alpha_channel: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	animated: Option<bool>,
	checksum: Uuid,
	game: i32,
	height: i32,
	image_id: String,
	url: String,
	width: i32,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
pub enum RatingCategory {
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
pub enum RatingEnum {
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
	id: i32,
	category: RatingCategory,
	checksum: Uuid,
	#[serde(skip_serializing_if = "Option::is_none")]
	content_descriptions: Option<Vec<i64>>,
	rating: RatingEnum,
	#[serde(skip_serializing_if = "Option::is_none")]
	rating_cover_url: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	synopsis: Option<String>,
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
	category: AgeRatingContentCategory,
	checksum: Uuid,
	description: String,
}
