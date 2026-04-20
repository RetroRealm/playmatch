use thiserror::Error;

pub const APICALYPSE_MAX_LITERAL_LEN: usize = 200;

#[derive(Debug, Error)]
pub enum ApicalypseLiteralError {
	#[error("apicalypse string literal exceeds {APICALYPSE_MAX_LITERAL_LEN} characters")]
	TooLong,
}

/// Returns `input` as a quoted Apicalypse string literal with `"` and `\`
/// escaped and control characters stripped. Length-capped so callers cannot
/// build unbounded upstream query bodies.
pub fn quote(input: &str) -> Result<String, ApicalypseLiteralError> {
	if input.chars().count() > APICALYPSE_MAX_LITERAL_LEN {
		return Err(ApicalypseLiteralError::TooLong);
	}

	let mut out = String::with_capacity(input.len() + 2);
	out.push('"');
	for c in input.chars() {
		if c.is_control() {
			continue;
		}
		match c {
			'\\' => out.push_str("\\\\"),
			'"' => out.push_str("\\\""),
			other => out.push(other),
		}
	}
	out.push('"');
	Ok(out)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn quotes_simple_string() {
		assert_eq!(quote("foo").unwrap(), "\"foo\"");
	}

	#[test]
	fn escapes_double_quote() {
		assert_eq!(quote(r#"foo"bar"#).unwrap(), r#""foo\"bar""#);
	}

	#[test]
	fn escapes_backslash() {
		assert_eq!(quote(r"foo\bar").unwrap(), r#""foo\\bar""#);
	}

	#[test]
	fn strips_control_characters() {
		assert_eq!(quote("foo\nbar\tbaz").unwrap(), "\"foobarbaz\"");
	}

	#[test]
	fn neutralizes_injection_payload() {
		let poison = r#"foo"; fields *; where id > 0;"#;
		let escaped = quote(poison).unwrap();
		assert_eq!(escaped, r#""foo\"; fields *; where id > 0;""#);
	}

	#[test]
	fn rejects_overlong_input() {
		let long = "a".repeat(APICALYPSE_MAX_LITERAL_LEN + 1);
		assert!(matches!(quote(&long), Err(ApicalypseLiteralError::TooLong)));
	}

	#[test]
	fn accepts_exact_max_length() {
		let max = "a".repeat(APICALYPSE_MAX_LITERAL_LEN);
		assert!(quote(&max).is_ok());
	}
}
