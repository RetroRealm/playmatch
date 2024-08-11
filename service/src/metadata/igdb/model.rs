use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use utoipa::ToSchema;

pub type GameResponses = Vec<Game>;

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
	id: i32,
	category: GameCategory,
	#[serde(with = "ts_seconds")]
	pub created_at: DateTime<Utc>,
	#[serde(skip_serializing_if = "Option::is_none")]
	external_games: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	expanded_games: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	expansions: Option<Vec<i32>>,
	name: String,
	slug: String,
	#[serde(with = "ts_seconds")]
	pub updated_at: DateTime<Utc>,
	url: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	websites: Option<Vec<i32>>,
	checksum: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	cover: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	game_modes: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	genres: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	involved_companies: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	summary: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	language_supports: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	bundles: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	first_release_date: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	forks: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	platforms: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	player_perspectives: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	release_dates: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	similar_games: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	tags: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	themes: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	artworks: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	screenshots: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	age_ratings: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	aggregated_rating: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	aggregated_rating_count: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	total_rating: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	total_rating_count: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	status: Option<GameStatus>,
	#[serde(skip_serializing_if = "Option::is_none")]
	videos: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	parent_game: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	game_engines: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	keywords: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	storyline: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	rating: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	rating_count: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	alternative_names: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	collection: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	collections: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	game_localizations: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	version_parent: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	version_title: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	follows: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	franchises: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	dlcs: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	hypes: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	multiplayer_modes: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	ports: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	remakes: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	remasters: Option<Vec<i32>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	standalone_expansions: Option<Vec<i32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
struct Genres {
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
struct Franchise {
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
enum ExternalGameCategory {
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
enum ExternalGameMedia {
	Digital = 1,
	Physical,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
struct ExternalGame {
	category: ExternalGameCategory,
	checksum: Uuid,
	countries: Vec<i32>,
	#[serde(with = "ts_seconds")]
	created_at: DateTime<Utc>,
	game: i32,
	media: ExternalGameMedia,
	name: String,
	platform: i32,
	uid: String,
	#[serde(with = "ts_seconds")]
	updated_at: DateTime<Utc>,
	url: String,
	year: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
struct Cover {
	alpha_channel: bool,
	animated: bool,
	checksum: Uuid,
	game: i32,
	game_localization: i32,
	height: i32,
	image_id: String,
	url: String,
	width: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
struct Collection {
	as_child_relations: Vec<i32>,
	as_parent_relations: Vec<i32>,
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
struct Artwork {
	alpha_channel: bool,
	animated: bool,
	checksum: Uuid,
	game: i32,
	height: i32,
	image_id: String,
	url: String,
	width: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
struct AlternativeName {
	checksum: Uuid,
	comment: String,
	game: i32,
	name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
enum RatingCategory {
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
enum RatingEnum {
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
struct AgeRating {
	category: RatingCategory,
	checksum: Uuid,
	content_descriptions: Vec<AgeRatingContentDescription>,
	rating: RatingEnum,
	rating_cover_url: String,
	synopsis: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, ToSchema)]
#[repr(u8)]
enum AgeRatingContentCategory {
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
struct AgeRatingContentDescription {
	category: AgeRatingContentCategory,
	checksum: Uuid,
	description: String,
}
