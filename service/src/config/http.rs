pub const X_VERSION_HEADER_API: &str = env!("CARGO_PKG_VERSION");

pub const REQWEST_DEFAULT_USER_AGENT: &str = concat!("playmatch/", env!("CARGO_PKG_VERSION"));
