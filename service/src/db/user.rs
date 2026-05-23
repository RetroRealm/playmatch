use entity::user::{ActiveModel, Model};
use hmac::{Hmac, KeyInit, Mac};
use sea_orm::prelude::Uuid;
use sea_orm::{ActiveModelTrait, ColumnTrait, IntoActiveModel, Set, TryIntoModel};
use sea_orm::{DatabaseConnection, QueryFilter};
use sea_orm::{DbConn, DbErr, EntityTrait};
use sha2::Sha256;
use std::sync::OnceLock;
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

/// Fresh API keys are exactly 72 printable-ASCII chars; anything else is shape-rejected
/// before any DB or crypto work.
const API_KEY_LEN: usize = 72;

/// Minimum pepper length in raw bytes. 16 bytes is the floor for HMAC-key entropy.
const MIN_PEPPER_BYTES: usize = 16;

static PEPPER: OnceLock<Vec<u8>> = OnceLock::new();

/// Install the API key pepper. Must be called exactly once at startup from
/// `api::start()` before any request is served. `pepper_hex` is the hex-encoded
/// output of `openssl rand -hex 32` or equivalent.
///
/// # Errors
/// Returns `Err` if the hex is malformed, the decoded pepper is shorter than
/// [`MIN_PEPPER_BYTES`], or this function has already been called.
pub fn init_pepper(pepper_hex: &str) -> Result<(), &'static str> {
	let pepper =
		hex::decode(pepper_hex.trim()).map_err(|_| "API_KEY_PEPPER must be hex-encoded")?;
	if pepper.len() < MIN_PEPPER_BYTES {
		return Err("API_KEY_PEPPER must decode to at least 16 bytes (32 hex chars)");
	}
	PEPPER
		.set(pepper)
		.map_err(|_| "API_KEY_PEPPER already initialised")
}

fn pepper() -> &'static [u8] {
	PEPPER
		.get()
		.expect("API_KEY_PEPPER not initialised; call init_pepper() at startup")
		.as_slice()
}

fn hash_api_key_hmac(token: &str) -> String {
	let mut mac = HmacSha256::new_from_slice(pepper()).expect("HMAC-SHA256 accepts any key length");
	mac.update(token.as_bytes());
	hex::encode(mac.finalize().into_bytes())
}

/// Dummy constant-time compare so miss/reject paths burn similar CPU to the hit path.
#[inline]
fn constant_time_burn() {
	let a = [0u8; 64];
	let _ = bool::from(a.ct_eq(&a));
}

pub async fn get_user_by_api_key(token: String, db_conn: &DbConn) -> Result<Option<Model>, DbErr> {
	// Rejecting malformed shapes before DB/crypto work closes oversized-token CPU
	// amplification and narrows the hit/miss timing channel.
	if token.len() != API_KEY_LEN || !token.chars().all(|c| c.is_ascii_graphic()) {
		constant_time_burn();
		return Ok(None);
	}

	let hmac_hash = hash_api_key_hmac(&token);
	if let Some(user) = entity::user::Entity::find()
		.filter(entity::user::Column::ApiKeyHashHmac.eq(&hmac_hash))
		.one(db_conn)
		.await?
	{
		return Ok(Some(user));
	}

	constant_time_burn();
	Ok(None)
}

/// Look up a user by their internal UUID.
pub async fn get_user_by_id(id: Uuid, db_conn: &DbConn) -> Result<Option<Model>, DbErr> {
	let user = entity::user::Entity::find_by_id(id).one(db_conn).await?;

	Ok(user)
}

/// Look up a user by their Discord id.
pub async fn get_user_by_discord_id(
	discord_id: i64,
	db_conn: &DbConn,
) -> Result<Option<Model>, DbErr> {
	let user = entity::user::Entity::find()
		.filter(entity::user::Column::DiscordId.eq(discord_id))
		.one(db_conn)
		.await?;

	Ok(user)
}

/// Set the permission level of the given user and persist the change.
pub async fn update_user_permission_level(
	user: Model,
	permissions: entity::sea_orm_active_enums::UserPermissionsEnum,
	db_conn: &DbConn,
) -> Result<Model, DbErr> {
	let mut active_model: entity::user::ActiveModel = user.into_active_model();
	active_model.permissions = Set(permissions);

	let updated_user = active_model.update(db_conn).await?;

	updated_user.try_into_model()
}

/// Insert a new user row and return the persisted model.
pub async fn insert_user(user: ActiveModel, db_conn: &DatabaseConnection) -> Result<Model, DbErr> {
	entity::user::Entity::insert(user)
		.exec_with_returning(db_conn)
		.await
}
