use crate::routes::company::__path_get_all_companies;
use crate::routes::health::{__path_health, __path_ready};
use crate::routes::identify::__path_identify;
use crate::routes::igdb::{
	__path_get_age_rating_by_id, __path_get_age_ratings_by_ids, __path_get_alternative_name_by_id,
	__path_get_alternative_names_by_ids, __path_get_artwork_by_id, __path_get_artworks_by_ids,
	__path_get_collection_by_id, __path_get_collections_by_ids, __path_get_cover_by_id,
	__path_get_covers_by_ids, __path_get_external_game_by_id, __path_get_external_games_by_ids,
	__path_get_franchise_by_id, __path_get_franchises_by_ids, __path_get_game_by_id,
	__path_get_games_by_ids, __path_get_genre_by_id, __path_get_genres_by_ids,
	__path_search_game_by_name,
};
use crate::routes::platform::__path_get_all_platforms;
use service::metadata::igdb::model::{
	AgeRating, AgeRatingCategory, AgeRatingContentCategory, AgeRatingContentDescription,
	AgeRatingEnum, AlternativeName, Artwork, Character, CharacterGender, CharacterSpecies,
	Collection, CollectionMembership, CollectionMembershipType, CollectionRelation,
	CollectionRelationType, CollectionType, Company, CompanyChangeDateCategory, CompanyLogo,
	CompanyStartDateCategory, CompanyWebsite, CompanyWebsiteCategory, Cover, Event, EventLogo,
	EventNetwork, ExternalGame, ExternalGameCategory, ExternalGameMedia, Franchise, Game,
	GameCategory, GameEngine, GameEngineLogo, GameLocalization, GameMode, GameStatus, GameVersion,
	GameVersionFeature, GameVersionFeatureCategory, GameVersionFeatureValue,
	GameVersionFeatureValueEnum, GameVideo, Genre, InvolvedCompany, Keyword, Language,
	LanguageSupport, LanguageSupportType, MultiplayerMode, NetworkType, Platform, PlatformCategory,
	PlatformFamily, PlatformLogo, PlatformVersion, PlatformVersionCompany,
	PlatformVersionReleaseDate, PlatformVersionReleaseDateCategory,
	PlatformVersionReleaseDateRegion, PlatformWebsite, PlatformWebsiteCategory, PlayerPerspective,
	PopularityPrimitive, PopularitySource, PopularityType, Region, ReleaseDate,
	ReleaseDateCategory, ReleaseDateRegion, ReleaseDateStatus, Screenshot, Theme, WebsiteCategory,
};
use service::model::{
	AutomaticMatchReason, CompanyResponse, ExternalMetadata, FailedMatchReason, GameMatchResult,
	GameMatchType, ManualMatchMode, MatchType, MetadataProvider, PlatformResponse,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
	paths(
		health,
		ready,
		identify,
		get_game_by_id,
		get_games_by_ids,
		search_game_by_name,
		get_age_rating_by_id,
		get_age_ratings_by_ids,
		get_alternative_name_by_id,
		get_alternative_names_by_ids,
		get_artwork_by_id,
		get_artworks_by_ids,
		get_collection_by_id,
		get_collections_by_ids,
		get_cover_by_id,
		get_covers_by_ids,
		get_external_game_by_id,
		get_external_games_by_ids,
		get_franchise_by_id,
		get_franchises_by_ids,
		get_genre_by_id,
		get_genres_by_ids,
		get_all_companies,
		get_all_platforms
	),
	components(schemas(
		GameMatchResult,
		CompanyResponse,
		PlatformResponse,
		GameMatchType,
		ExternalMetadata,
		MatchType,
		ManualMatchMode,
		FailedMatchReason,
		MetadataProvider,
		AutomaticMatchReason,
		AgeRating,
		AgeRatingCategory,
		AgeRatingContentCategory,
		AgeRatingContentDescription,
		AgeRatingEnum,
		AlternativeName,
		Artwork,
		Character,
		CharacterGender,
		CharacterSpecies,
		Collection,
		CollectionMembership,
		CollectionMembershipType,
		CollectionRelation,
		CollectionRelationType,
		CollectionType,
		Company,
		CompanyChangeDateCategory,
		CompanyLogo,
		CompanyStartDateCategory,
		CompanyWebsite,
		CompanyWebsiteCategory,
		Cover,
		Event,
		EventLogo,
		EventNetwork,
		ExternalGame,
		ExternalGameCategory,
		ExternalGameMedia,
		Franchise,
		Game,
		GameCategory,
		GameEngine,
		GameEngineLogo,
		GameLocalization,
		GameMode,
		GameStatus,
		GameVersion,
		GameVersionFeature,
		GameVersionFeatureCategory,
		GameVersionFeatureValue,
		GameVersionFeatureValueEnum,
		GameVideo,
		Genre,
		InvolvedCompany,
		Keyword,
		Language,
		LanguageSupport,
		LanguageSupportType,
		MultiplayerMode,
		NetworkType,
		Platform,
		PlatformCategory,
		PlatformFamily,
		PlatformLogo,
		PlatformVersion,
		PlatformVersionCompany,
		PlatformVersionReleaseDate,
		PlatformVersionReleaseDateCategory,
		PlatformVersionReleaseDateRegion,
		PlatformWebsite,
		PlatformWebsiteCategory,
		PlayerPerspective,
		PopularityPrimitive,
		PopularitySource,
		PopularityType,
		Region,
		ReleaseDate,
		ReleaseDateCategory,
		ReleaseDateRegion,
		ReleaseDateStatus,
		Screenshot,
		Theme,
		WebsiteCategory
	))
)]
pub struct ApiDoc;
