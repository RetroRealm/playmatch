use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
	static ref BRACKET_REGEX: Regex = Regex::new(r"\s*\(.*?\)").unwrap();
}

pub(crate) fn clean_name(input: &str) -> String {
	BRACKET_REGEX.replace_all(input, "").to_string()
}

pub fn normalize_title(input: &str) -> String {
	lazy_static! {
		static ref RE_STRIP: Regex = Regex::new(r" - |: ").unwrap();
		static ref RE_LEADING: Regex = Regex::new(r"^(?i)(the |a |an )").unwrap();
		// Remove ", The", ", A", ", An" anywhere in the string (case-insensitive)
		static ref RE_ARTICLE: Regex = Regex::new(r"(?i),\s*(the|a|an)\b").unwrap();
		static ref RE_ROMAN: Regex = Regex::new(
			r"\b(?i:M{0,4}(CM|CD|D?C{0,3})(XC|XL|L?X{0,3})(IX|IV|V?I{0,3}))\b"
		).unwrap();
	}

	// 1. Strip " - " and ": "
	let mut s = RE_STRIP.replace_all(input, " ").to_string();

	// 2. Strip leading article
	s = RE_LEADING.replace(&s, "").to_string();

	// 3. Remove all article suffixes (anywhere in string)
	s = RE_ARTICLE.replace_all(&s, "").to_string();

	// 4. Replace all roman numerals
	s = RE_ROMAN
		.replace_all(&s, |caps: &regex::Captures| {
			let roman = &caps[0];
			if !roman.is_empty()
				&& let Some(val) = roman_to_int(roman)
			{
				return val.to_string();
			}
			roman.to_string()
		})
		.to_string();

	s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Convert a Roman numeral (up to 3999) to an integer.
pub(crate) fn roman_to_int(roman: &str) -> Option<u32> {
	let mut result = 0;
	let mut prev = 0;
	for c in roman.chars().rev() {
		let val = match c {
			'I' | 'i' => 1,
			'V' | 'v' => 5,
			'X' | 'x' => 10,
			'L' | 'l' => 50,
			'C' | 'c' => 100,
			'D' | 'd' => 500,
			'M' | 'm' => 1000,
			_ => return None,
		};
		if val < prev {
			result -= val;
		} else {
			result += val;
		}
		prev = val;
	}
	Some(result)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_normalize_title() {
		assert_eq!(
			normalize_title("The Legend of Zelda: Majora's Mask"),
			"Legend of Zelda Majora's Mask"
		);
		assert_eq!(
			normalize_title("Star Wars IV: A New Hope"),
			"Star Wars 4 A New Hope"
		);
		assert_eq!(
			normalize_title("A Series of Unfortunate Events, The"),
			"Series of Unfortunate Events"
		);
		assert_eq!(
			normalize_title("An American Tail - Fievel Goes West"),
			"American Tail Fievel Goes West"
		);
		assert_eq!(normalize_title("Final Fantasy VII"), "Final Fantasy 7");
		assert_eq!(normalize_title("Rocky II"), "Rocky 2");
		assert_eq!(normalize_title("An Untitled Story"), "Untitled Story");
		assert_eq!(
			normalize_title("Legend of Zelda, The - Twilight Princess"),
			"Legend of Zelda Twilight Princess"
		);
		assert_eq!(
			normalize_title("The Legend of Zelda: Twilight Princess"),
			"Legend of Zelda Twilight Princess"
		)
	}
}
