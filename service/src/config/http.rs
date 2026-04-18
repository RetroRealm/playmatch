use std::sync::LazyLock;

pub static X_VERSION_HEADER_API: LazyLock<String> = LazyLock::new(|| {
	std::env::var("X_VERSION_HEADER_API").unwrap_or_else(|_| "unknown".to_string())
});

pub static REQWEST_DEFAULT_USER_AGENT: LazyLock<String> = LazyLock::new(|| {
	std::env::var("REQWEST_DEFAULT_USER_AGENT").unwrap_or_else(|_| "unknown".to_string())
});
