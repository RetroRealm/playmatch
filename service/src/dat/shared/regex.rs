use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
	pub static ref DAT_TAG_REGEX: Regex = Regex::new(r"\(([^)]+)\)").unwrap();
}
