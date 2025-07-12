use rand::Rng;
use rand::distr::Alphanumeric;

pub fn random_sized_string(size: usize) -> String {
	rand::rng()
		.sample_iter(&Alphanumeric)
		.take(size)
		.map(char::from)
		.collect()
}
