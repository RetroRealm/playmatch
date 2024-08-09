use service::http;

pub mod built_info {
	// The file has been placed there by the build script.
	include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

fn main() {
	// Safety: This is only mutated in the main function
	unsafe {
		http::abstraction::USER_AGENT =
			format!("{}/{}", built_info::PKG_NAME, built_info::PKG_VERSION)
	}

	api::main();
}
