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
use service::metadata::igdb::model::{
	AgeRating, AgeRatingContentCategory, AgeRatingContentDescription, AlternativeName, Artwork,
	Collection, Cover, ExternalGame, ExternalGameCategory, ExternalGameMedia, Franchise, Game,
	GameCategory, GameStatus, Genre, RatingCategory, RatingEnum,
};
use service::model::GameMatchResult;
use service::model::GameMatchType;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
	paths(
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
		get_genres_by_ids
	),
	components(schemas(
		GameMatchResult,
		GameMatchType,
		Game,
		GameCategory,
		GameStatus,
		AgeRating,
		AlternativeName,
		RatingCategory,
		AgeRatingContentDescription,
		AgeRatingContentCategory,
		RatingEnum,
		Artwork,
		Collection,
		Cover,
		ExternalGame,
		Franchise,
		Genre,
		ExternalGameCategory,
		ExternalGameMedia
	))
)]
pub struct ApiDoc;
