pub mod http;

use lazy_static::lazy_static;

lazy_static! {
	pub static ref PARALLELISM: usize = std::env::var("PARALLELISM")
		.unwrap_or_else(|_| CPU_COUNT.to_string())
		.parse()
		.expect("PARALLELISM must be a number");
	pub static ref CPU_COUNT: usize = num_cpus::get();
}
