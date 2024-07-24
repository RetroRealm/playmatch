use actix_web::{HttpResponse, post, Responder, web};
use actix_web::http::header::{CacheControl, CacheDirective};
use actix_web::web::Data;
use log::debug;
use sea_orm::DatabaseConnection;

use service::query::find_game_release_and_id_mapping_by_md5;

use crate::models::game_file::{GameFileRequest, GameMatchResponse, GameMatchType};

#[utoipa::path(
	context_path = "/api",
	responses(
		(status = 200, description = "Returns info about a possible match via hashes or filename and size", body = GameMatchResponse)
	)
)]
#[post("/identify/ids")]
pub async fn identify(
    body: web::Json<GameFileRequest>,
    db_conn: Data<DatabaseConnection>,
) -> crate::error::Result<impl Responder> {
    debug!("Received request: {:?}", body);

    let mut builder = HttpResponse::Ok();

    let mut response_body = None;

    if let Some(md5) = &body.md5 {
        if let Some((game_release, game_release_id_mapping)) =
            find_game_release_and_id_mapping_by_md5(md5, db_conn.get_ref()).await?
        {
            response_body = Some(GameMatchResponse {
                game_match_type: GameMatchType::MD5,
                playmatch_id: Some(game_release.id.clone()),
                igdb_id: Some(game_release_id_mapping.igdb_id),
                mobygames_id: game_release_id_mapping.moby_games_id,
            })
        }
    }

    builder.insert_header(CacheControl(vec![
        CacheDirective::Public,
        CacheDirective::MaxAge(7200u32),
    ]));

    Ok(
        builder.json(response_body.unwrap_or_else(|| GameMatchResponse {
            game_match_type: GameMatchType::NoMatch,
            playmatch_id: None,
            igdb_id: None,
            mobygames_id: None,
        })),
    )
}
