pub mod http;
pub mod versions;

use lazy_static::lazy_static;

lazy_static! {
	pub static ref PARALLELISM: usize = std::env::var("PARALLELISM")
		.unwrap_or_else(|_| CPU_COUNT.to_string())
		.parse()
		.expect("PARALLELISM must be a positive integer (for example 4)");
	pub static ref CPU_COUNT: usize = num_cpus::get();
}
