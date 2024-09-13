pub mod http;

use lazy_static::lazy_static;

lazy_static! {
	pub static ref PARALLELISM: usize = num_cpus::get();
}
