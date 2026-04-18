use rand::distr::{Alphanumeric, SampleString};

pub fn random_sized_string(size: usize) -> String {
	Alphanumeric.sample_string(&mut rand::rng(), size)
}
