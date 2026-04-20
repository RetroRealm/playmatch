use actix_web::web::Data;
use actix_web::{HttpResponse, Responder, get};
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Cache window for a successful readiness probe to absorb bursty scrapes.
const READY_CACHE_SECONDS: i64 = 5;

static LAST_READY_AT: AtomicI64 = AtomicI64::new(0);

fn unix_now() -> i64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(|d| d.as_secs() as i64)
		.unwrap_or(0)
}

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

#[utoipa::path(
	get,
	context_path = "/api",
	tag = "Health",
	responses(
		(status = 200, description = "Service is Ready"),
		(status = 503, description = "Service is not ready")
	)
)]
#[get("/ready")]
pub async fn ready(
	db_conn: Data<DatabaseConnection>,
	redis_client: Data<redis::Client>,
) -> impl Responder {
	let now = unix_now();
	let last = LAST_READY_AT.load(Ordering::Relaxed);
	if last != 0 && now.saturating_sub(last) < READY_CACHE_SECONDS {
		return HttpResponse::Ok().body("Ready");
	}

	if let Err(err) = db_conn
		.execute(Statement::from_string(
			db_conn.get_database_backend(),
			"SELECT 1".to_string(),
		))
		.await
	{
		return HttpResponse::ServiceUnavailable().body(format!("database not ready: {err}"));
	}

	let mut redis_conn = match redis_client.get_multiplexed_async_connection().await {
		Ok(c) => c,
		Err(err) => {
			return HttpResponse::ServiceUnavailable().body(format!("redis not ready: {err}"));
		}
	};

	if let Err(err) = redis::cmd("PING")
		.query_async::<String>(&mut redis_conn)
		.await
	{
		return HttpResponse::ServiceUnavailable().body(format!("redis not ready: {err}"));
	}

	LAST_READY_AT.store(now, Ordering::Relaxed);
	HttpResponse::Ok().body("Ready")
}
