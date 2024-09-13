use lazy_static::lazy_static;

lazy_static! {
	pub static ref X_VERSION_HEADER_API: String =
		std::env::var("X_VERSION_HEADER_API").unwrap_or_else(|_| "unknown".to_string());
	pub static ref REQWEST_DEFAULT_USER_AGENT: String =
		std::env::var("REQWEST_DEFAULT_USER_AGENT").unwrap_or_else(|_| "unknown".to_string());
}
