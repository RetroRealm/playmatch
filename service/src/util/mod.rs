use rand::distr::Alphanumeric;
use rand::Rng;

pub fn random_sized_string(size: usize) -> String {
	rand::rng()
		.sample_iter(&Alphanumeric)
		.take(size)
		.map(char::from)
		.collect()
}
