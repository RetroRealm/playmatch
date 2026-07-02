//! Public API DTOs shared by the api and mcp crates. Response types follow a
//! twin-struct convention: the V1 shapes are frozen, and each has a `*V2`
//! twin that is free to evolve; doc text on twins is kept in lockstep when
//! the meaning is identical.

pub mod external_suggestion;
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
			return Err(format!("{label} must be {required_len} hex characters"));
		}
		if !v.chars().all(|c| c.is_ascii_hexdigit()) {
			return Err(format!("{label} must be hexadecimal"));
		}
	}
	Ok(())
}
