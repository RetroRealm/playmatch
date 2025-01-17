use dotenvy::dotenv;
use env_logger::Env;
use log::info;

pub mod built_info {
	// The file has been placed there by the build script.
	include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

fn main() {
	// Load environment variables from .env file, if present but do nothing if it fails
	let _ = dotenv();
	env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

	info!(
		"Starting {} v{} ({}) built on {}",
		built_info::PKG_NAME,
		built_info::PKG_VERSION,
		built_info::GIT_COMMIT_HASH.unwrap_or("build commit unknown"),
		built_info::BUILT_TIME_UTC
	);

	std::env::set_var(
		"REQWEST_DEFAULT_USER_AGENT",
		format!("{}/{}", built_info::PKG_NAME, built_info::PKG_VERSION),
	);
	std::env::set_var("X_VERSION_HEADER_API", built_info::PKG_VERSION);

	api::main();
}
