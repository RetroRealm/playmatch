use actix_web::{get, HttpResponse, Responder, web};
use log::debug;

use crate::models::game_file::{GameFileRequest, GameMatchResponse, GameMatchType};

#[utoipa::path(
	context_path = "/api",
	responses(
		(status = 200, description = "Returns info about a possible match via hashes or filename and size", body = GameMatchResponse)
	)
)]
#[get("/identify")]
pub async fn hello_world(body: web::Json<GameFileRequest>) -> impl Responder {
    debug!("Received request: {:?}", body);

    HttpResponse::Ok().json(GameMatchResponse {
        game_match_type: GameMatchType::NoMatch,
        playmatch_id: None,
        igdb_id: None,
        mobygames_id: None,
    })
}
