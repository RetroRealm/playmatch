pub mod matching;
pub mod suggestion;
pub mod user;

mod match_result;
mod metadata;
mod playmatch;

pub use match_result::*;
pub use metadata::*;
pub use playmatch::*;

pub const MAX_NAME_INPUT_LEN: usize = 512;

pub(crate) fn validate_optional_hex(
	value: &Option<String>,
	required_len: usize,
	label: &str,
) -> Result<(), String> {
	if let Some(v) = value {
		if v.len() != required_len {
			return Err(format!(
				"{label} must be exactly {required_len} hex characters"
			));
		}
		if !v.chars().all(|c| c.is_ascii_hexdigit()) {
			return Err(format!("{label} must be hex-encoded"));
		}
	}
	Ok(())
}
