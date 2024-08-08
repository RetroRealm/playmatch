use actix_web::{get, HttpResponse, post, Responder, web};
use actix_web::web::Data;
use log::debug;
use sea_orm::DatabaseConnection;

use service::db::query::find_game_release_and_id_mapping_by_md5;

use crate::abstraction::cache::InsertCacheHeaders;
use crate::error;
use crate::models::game_file::{
	GameFileRequest, GameMatchResponse, GameMatchResponseBuilder, GameMatchType,
};
use crate::models::game_file::GameMatchType::MD5;

#[utoipa::path(
	context_path = "/api",
	responses(
		(status = 200, description = "Returns info about a possible match via hashes or filename and size", body = GameMatchResponse)
	)
)]
#[get("/identify/ids")]
pub async fn identify(
    body: web::Query<GameFileRequest>,
    db_conn: Data<DatabaseConnection>,
) -> error::Result<impl Responder> {
    debug!("Received request: {:?}", body);

    let mut builder = HttpResponse::Ok();

    let mut response_body = None;

    if let Some(md5) = &body.md5 {
        if let Some((game_release, game_release_id_mapping)) =
            find_game_release_and_id_mapping_by_md5(md5, db_conn.get_ref()).await?
        {
            response_body = Some(
                GameMatchResponseBuilder::default()
                    .game_match_type(MD5)
                    .igdb_id(Some(game_release_id_mapping.igdb_id))
                    .mobygames_id(game_release_id_mapping.moby_games_id)
                    .playmatch_id(Some(game_release.id.clone()))
                    .build()?,
            );
        }
    }

    builder.set_public_cacheable(7200u32);

    Ok(
        builder.json(response_body.unwrap_or_else(|| GameMatchResponse {
            game_match_type: GameMatchType::NoMatch,
            playmatch_id: None,
            igdb_id: None,
            mobygames_id: None,
        })),
    )
}
