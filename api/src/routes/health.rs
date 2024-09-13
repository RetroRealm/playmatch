use actix_web::{get, HttpResponse, Responder};

/// Checks if the service is healthy
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "Health",
	responses(
		(status = 200, description = "Service is healthy")
	)
)]
#[get("/health")]
pub async fn health() -> impl Responder {
	HttpResponse::Ok().body("Healthy")
}

/// Checks if the service is ready
#[utoipa::path(
	get,
	context_path = "/api",
	tag = "Health",
	responses(
		(status = 200, description = "Service is Ready")
	)
)]
#[get("/ready")]
pub async fn ready() -> impl Responder {
	HttpResponse::Ok().body("Ready")
}
