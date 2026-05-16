use lazy_static::lazy_static;
use regex::Regex;
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

pub(crate) fn clean_name(input: &str) -> String {
	crate::matching::name_parse::parse_name(input).base
}

pub fn normalize_title(input: &str) -> String {
	lazy_static! {
		static ref RE_AMPERSAND: Regex = Regex::new(r"\s*&\s*").unwrap();
		static ref RE_STRIP: Regex = Regex::new(r" - |: ").unwrap();
		// Localised leading articles. Single-letter forms in the added locales
		// (PT "o", IT "i") are deliberately omitted to avoid clobbering Roman
		// numerals and short proper names; the original English "a"/"an" stay
		// for backwards compatibility.
		// Positional regex strips occurrences at the start of the title even
		// when the word happens to be part of a proper name (e.g. "La Mulana"
		// loses "La "); accepted false-positive in exchange for rescuing
		// localised DAT releases that only differ from sibling providers'
		// titles by a leading article.
		static ref RE_LEADING: Regex = Regex::new(
			r"^(?i)(the |a |an |der |die |das |le |la |les |el |los |las |il |lo |gli |os |as )"
		)
		.unwrap();
		static ref RE_ARTICLE: Regex = Regex::new(
			r"(?i),\s*(the|a|an|der|die|das|le|la|les|el|los|las|il|lo|gli|os|as)\b"
		)
		.unwrap();
		static ref RE_ROMAN: Regex =
			Regex::new(r"\b(?i:M{0,4}(CM|CD|D?C{0,3})(XC|XL|L?X{0,3})(IX|IV|V?I{0,3}))\b").unwrap();
	}

	// ™ NFKD-decomposes to "TM" which would later collide with titles
	// like "TMNT", so strip the glyphs before NFKD.
	let pre = input.replace(['\u{2122}', '\u{00ae}', '\u{00a9}'], "");

	let mut s: String = pre.nfkd().filter(|c| !is_combining_mark(*c)).collect();

	s = s.replace(['\u{2018}', '\u{2019}', '\u{0060}'], "'");

	s = RE_AMPERSAND.replace_all(&s, " and ").to_string();

	s = RE_STRIP.replace_all(&s, " ").to_string();

	s = RE_LEADING.replace(&s, "").to_string();

	s = RE_ARTICLE.replace_all(&s, "").to_string();

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

	#[test]
	fn diacritic_fold() {
		assert_eq!(normalize_title("Pokémon Red"), "Pokemon Red");
		assert_eq!(normalize_title("Café del Mar"), "Cafe del Mar");
		assert_eq!(normalize_title("Pokémon"), normalize_title("Pokemon"));
	}

	#[test]
	fn localised_leading_articles() {
		assert_eq!(normalize_title("Der Herr der Ringe"), "Herr der Ringe");
		assert_eq!(normalize_title("Die Brücke"), "Brucke");
		assert_eq!(normalize_title("Das Boot"), "Boot");
		assert_eq!(normalize_title("Le Petit Prince"), "Petit Prince");
		assert_eq!(normalize_title("La Cité"), "Cite");
		assert_eq!(normalize_title("Les Misérables"), "Miserables");
		assert_eq!(normalize_title("El Quijote"), "Quijote");
		assert_eq!(normalize_title("Los Angeles"), "Angeles");
		assert_eq!(normalize_title("Las Vegas"), "Vegas");
		assert_eq!(normalize_title("Il Padrino"), "Padrino");
		assert_eq!(normalize_title("Lo Hobbit"), "Hobbit");
		assert_eq!(normalize_title("Gli Anelli"), "Anelli");
		assert_eq!(normalize_title("Os Lusíadas"), "Lusiadas");
		assert_eq!(normalize_title("As Aventuras"), "Aventuras");
	}

	#[test]
	fn localised_trailing_articles() {
		assert_eq!(normalize_title("Petit Prince, Le"), "Petit Prince");
		assert_eq!(normalize_title("Quijote, El"), "Quijote");
		assert_eq!(normalize_title("Padrino, Il"), "Padrino");
		assert_eq!(normalize_title("Boot, Das"), "Boot");
	}

	#[test]
	fn single_letter_locale_articles_left_alone() {
		// Italian "i" and Portuguese "o" leading articles are intentionally
		// not added; verifies a Roman-numeral "I" is still handled by the
		// numeral conversion rather than mistakenly stripped as an article,
		// and a single capital "O" word stays in place.
		assert_eq!(normalize_title("Final Fantasy I"), "Final Fantasy 1");
		assert_eq!(normalize_title("O Holy Night"), "O Holy Night");
	}

	#[test]
	fn trademark_strip() {
		assert_eq!(normalize_title("Madden NFL™"), "Madden NFL");
		assert_eq!(normalize_title("Tetris®"), "Tetris");
		assert_eq!(normalize_title("Pong© 2"), "Pong 2");
	}

	#[test]
	fn apostrophe_variants_collapse() {
		let curly = normalize_title("Majora\u{2019}s Mask");
		let backtick = normalize_title("Majora\u{0060}s Mask");
		let straight = normalize_title("Majora's Mask");
		assert_eq!(curly, straight);
		assert_eq!(backtick, straight);
	}

	#[test]
	fn ampersand_to_and() {
		assert_eq!(
			normalize_title("Sonic & Knuckles"),
			normalize_title("Sonic and Knuckles")
		);
		assert_eq!(normalize_title("Sonic & Knuckles"), "Sonic and Knuckles");
	}

	#[test]
	fn ampersand_no_space() {
		assert_eq!(normalize_title("Tom&Jerry"), "Tom and Jerry");
	}

	#[test]
	fn combined_diacritic_apostrophe_trademark() {
		let a = normalize_title("Pokémon™ Red’s Adventure");
		let b = normalize_title("Pokemon Red's Adventure");
		assert_eq!(a, b);
	}

	#[test]
	fn clean_name_strips_parens() {
		assert_eq!(clean_name("Mario (USA)"), "Mario");
		assert_eq!(clean_name("Mario (USA) (Rev A)"), "Mario");
		assert_eq!(clean_name("Plain Title"), "Plain Title");
	}

	#[test]
	fn clean_name_handles_nested_parens() {
		assert_eq!(clean_name("Game (Foo (Bar))"), "Game");
	}

	#[test]
	fn clean_name_strips_hack_pirate_unl_tags() {
		assert_eq!(clean_name("Mario (USA) (Hack)"), "Mario");
		assert_eq!(clean_name("Tetris (Unl)"), "Tetris");
		assert_eq!(clean_name("Sonic (Pirate)"), "Sonic");
	}
}
