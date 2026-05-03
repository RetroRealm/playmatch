use lazy_static::lazy_static;
use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedName {
	pub base: String,
	pub regions: Vec<RegionTag>,
	pub year: Option<u16>,
	pub revision: Option<String>,
	pub variant: Option<Variant>,
	pub disc: Option<u8>,
	pub languages: Vec<Lang>,
	pub residual: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegionTag {
	World,
	Usa,
	Europe,
	Japan,
	Asia,
	Australia,
	Brazil,
	Korea,
	China,
	Germany,
	France,
	Italy,
	Spain,
	Netherlands,
}

impl RegionTag {
	pub fn ss_codes(&self) -> &'static [&'static str] {
		match self {
			RegionTag::World => &["wor"],
			RegionTag::Usa => &["us"],
			RegionTag::Europe => &["eu"],
			RegionTag::Japan => &["jp"],
			RegionTag::Asia => &["asi", "cn", "ko"],
			RegionTag::Australia => &["au"],
			RegionTag::Brazil => &["br"],
			RegionTag::Korea => &["ko"],
			RegionTag::China => &["cn"],
			RegionTag::Germany => &["de"],
			RegionTag::France => &["fr"],
			RegionTag::Italy => &[],
			RegionTag::Spain => &[],
			RegionTag::Netherlands => &[],
		}
	}

	pub fn lb_codes(&self) -> &'static [&'static str] {
		match self {
			RegionTag::World => &["World"],
			RegionTag::Usa => &["United States", "USA", "North America"],
			RegionTag::Europe => &["Europe", "European Union"],
			RegionTag::Japan => &["Japan"],
			RegionTag::Asia => &["Asia"],
			RegionTag::Australia => &["Australia", "Oceania"],
			RegionTag::Brazil => &["Brazil"],
			RegionTag::Korea => &["South Korea", "Korea"],
			RegionTag::China => &["China"],
			RegionTag::Germany => &["Germany"],
			RegionTag::France => &["France"],
			RegionTag::Italy => &["Italy"],
			RegionTag::Spain => &["Spain"],
			RegionTag::Netherlands => &["Netherlands"],
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
	Proto,
	Beta,
	Sample,
	Demo,
	Unlicensed,
	Pirate,
	Hack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lang {
	En,
	Fr,
	De,
	Es,
	It,
	Ja,
	Pt,
	Nl,
	Sv,
	Da,
	No,
	Fi,
	Ko,
	Zh,
	Ru,
	Pl,
	Cs,
	Tr,
}

pub fn parse_name(input: &str) -> ParsedName {
	let mut parsed = ParsedName::default();
	let mut base = String::with_capacity(input.len());

	let mut idx = 0;
	while idx < input.len() {
		let rest = &input[idx..];
		let next_char = rest.chars().next().expect("non-empty rest");
		match next_char {
			'(' => {
				if let Some(end) = find_matching(input, idx, '(', ')') {
					let content = &input[idx + '('.len_utf8()..end];
					classify_into(content, &mut parsed);
					idx = end + ')'.len_utf8();
					continue;
				}
				base.push(next_char);
				idx += next_char.len_utf8();
			}
			'[' => {
				if let Some(end) = find_matching(input, idx, '[', ']') {
					let content = &input[idx + '['.len_utf8()..end];
					let trimmed = content.trim();
					if !trimmed.is_empty() {
						parsed.residual.push(trimmed.to_string());
					}
					idx = end + ']'.len_utf8();
					continue;
				}
				base.push(next_char);
				idx += next_char.len_utf8();
			}
			_ => {
				base.push(next_char);
				idx += next_char.len_utf8();
			}
		}
	}

	parsed.base = base.split_whitespace().collect::<Vec<_>>().join(" ");
	parsed
}

fn find_matching(s: &str, start: usize, open: char, close: char) -> Option<usize> {
	let mut depth = 0i32;
	for (offset, ch) in s[start..].char_indices() {
		if ch == open {
			depth += 1;
		} else if ch == close {
			depth -= 1;
			if depth == 0 {
				return Some(start + offset);
			}
		}
	}
	None
}

fn classify_into(content: &str, parsed: &mut ParsedName) {
	let trimmed = content.trim();
	if trimmed.is_empty() {
		return;
	}

	if let Some(v) = classify_variant(trimmed) {
		parsed.variant = Some(v);
		return;
	}
	if let Some(r) = classify_revision(trimmed) {
		parsed.revision = Some(r);
		return;
	}
	if let Some(d) = classify_disc(trimmed) {
		parsed.disc = Some(d);
		return;
	}
	if let Some(y) = classify_year(trimmed) {
		parsed.year = Some(y);
		return;
	}

	let tokens: Vec<&str> = trimmed.split(',').map(str::trim).collect();

	let regions: Option<Vec<_>> = tokens.iter().map(|t| classify_region(t)).collect();
	if let Some(rs) = regions {
		for r in rs {
			if !parsed.regions.contains(&r) {
				parsed.regions.push(r);
			}
		}
		return;
	}

	let langs: Option<Vec<_>> = tokens.iter().map(|t| classify_lang(t)).collect();
	if let Some(ls) = langs {
		for l in ls {
			if !parsed.languages.contains(&l) {
				parsed.languages.push(l);
			}
		}
		return;
	}

	parsed.residual.push(trimmed.to_string());
}

fn classify_variant(content: &str) -> Option<Variant> {
	match content {
		"Proto" | "Prototype" => Some(Variant::Proto),
		"Beta" => Some(Variant::Beta),
		"Sample" => Some(Variant::Sample),
		"Demo" => Some(Variant::Demo),
		"Unl" | "Unlicensed" => Some(Variant::Unlicensed),
		"Pirate" => Some(Variant::Pirate),
		"Hack" => Some(Variant::Hack),
		_ => None,
	}
}

fn classify_revision(content: &str) -> Option<String> {
	lazy_static! {
		static ref RE_REV: Regex = Regex::new(r"^Rev\s+\S+$").unwrap();
		static ref RE_VER: Regex = Regex::new(r"(?i)^v\d+(\.\d+)*[a-z]?$").unwrap();
	}
	if RE_REV.is_match(content) || RE_VER.is_match(content) {
		Some(content.to_string())
	} else {
		None
	}
}

fn classify_disc(content: &str) -> Option<u8> {
	lazy_static! {
		static ref RE_DISC: Regex = Regex::new(r"^(?:Disc|Disk)\s+(\d+)").unwrap();
		static ref RE_SIDE: Regex = Regex::new(r"^Side\s+([A-Z])$").unwrap();
	}
	if let Some(c) = RE_DISC.captures(content)
		&& let Some(m) = c.get(1)
		&& let Ok(n) = m.as_str().parse::<u8>()
	{
		return Some(n);
	}
	if let Some(c) = RE_SIDE.captures(content)
		&& let Some(m) = c.get(1)
		&& let Some(ch) = m.as_str().chars().next()
	{
		return Some((ch as u8) - b'A' + 1);
	}
	None
}

fn classify_year(content: &str) -> Option<u16> {
	let parsed = content.parse::<u16>().ok()?;
	(1970..=2100).contains(&parsed).then_some(parsed)
}

fn classify_region(token: &str) -> Option<RegionTag> {
	match token {
		"USA" | "U" | "United States" => Some(RegionTag::Usa),
		"Europe" | "E" | "EU" => Some(RegionTag::Europe),
		"Japan" | "J" => Some(RegionTag::Japan),
		"World" | "W" => Some(RegionTag::World),
		"Asia" => Some(RegionTag::Asia),
		"Australia" => Some(RegionTag::Australia),
		"Brazil" => Some(RegionTag::Brazil),
		"Korea" => Some(RegionTag::Korea),
		"China" => Some(RegionTag::China),
		"Germany" => Some(RegionTag::Germany),
		"France" => Some(RegionTag::France),
		"Italy" => Some(RegionTag::Italy),
		"Spain" => Some(RegionTag::Spain),
		"Netherlands" => Some(RegionTag::Netherlands),
		_ => None,
	}
}

fn classify_lang(token: &str) -> Option<Lang> {
	let base = token.split('-').next().unwrap_or(token).trim();
	match base {
		"En" => Some(Lang::En),
		"Fr" => Some(Lang::Fr),
		"De" => Some(Lang::De),
		"Es" => Some(Lang::Es),
		"It" => Some(Lang::It),
		"Ja" => Some(Lang::Ja),
		"Pt" => Some(Lang::Pt),
		"Nl" => Some(Lang::Nl),
		"Sv" => Some(Lang::Sv),
		"Da" => Some(Lang::Da),
		"No" => Some(Lang::No),
		"Fi" => Some(Lang::Fi),
		"Ko" => Some(Lang::Ko),
		"Zh" => Some(Lang::Zh),
		"Ru" => Some(Lang::Ru),
		"Pl" => Some(Lang::Pl),
		"Cs" => Some(Lang::Cs),
		"Tr" => Some(Lang::Tr),
		_ => None,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn plain_name_no_parens() {
		let p = parse_name("Super Mario Bros.");
		assert_eq!(p.base, "Super Mario Bros.");
		assert!(p.regions.is_empty());
		assert!(p.languages.is_empty());
		assert!(p.year.is_none());
	}

	#[test]
	fn single_region() {
		let p = parse_name("Super Mario Bros. (USA)");
		assert_eq!(p.base, "Super Mario Bros.");
		assert_eq!(p.regions, vec![RegionTag::Usa]);
	}

	#[test]
	fn multi_region_comma_space() {
		let p = parse_name("Super Mario Bros. (USA, Europe)");
		assert_eq!(p.base, "Super Mario Bros.");
		assert_eq!(p.regions, vec![RegionTag::Usa, RegionTag::Europe]);
	}

	#[test]
	fn multi_region_no_space() {
		let p = parse_name("Super Mario Bros. (USA,Europe)");
		assert_eq!(p.regions, vec![RegionTag::Usa, RegionTag::Europe]);
	}

	#[test]
	fn region_plus_languages() {
		let p = parse_name("Mario Kart DS (USA) (En,Fr,De,Es,It)");
		assert_eq!(p.base, "Mario Kart DS");
		assert_eq!(p.regions, vec![RegionTag::Usa]);
		assert_eq!(
			p.languages,
			vec![Lang::En, Lang::Fr, Lang::De, Lang::Es, Lang::It]
		);
	}

	#[test]
	fn revision_rev_a() {
		let p = parse_name("Tetris (USA) (Rev A)");
		assert_eq!(p.base, "Tetris");
		assert_eq!(p.regions, vec![RegionTag::Usa]);
		assert_eq!(p.revision.as_deref(), Some("Rev A"));
	}

	#[test]
	fn revision_v() {
		let p = parse_name("Doom (v1.1)");
		assert_eq!(p.base, "Doom");
		assert_eq!(p.revision.as_deref(), Some("v1.1"));
	}

	#[test]
	fn proto() {
		let p = parse_name("Mother 3 (Proto)");
		assert_eq!(p.variant, Some(Variant::Proto));
	}

	#[test]
	fn beta() {
		let p = parse_name("Sonic (Beta)");
		assert_eq!(p.variant, Some(Variant::Beta));
	}

	#[test]
	fn unlicensed() {
		let p = parse_name("Tetris (Unl)");
		assert_eq!(p.variant, Some(Variant::Unlicensed));
	}

	#[test]
	fn disc_number() {
		let p = parse_name("Final Fantasy VII (USA) (Disc 1)");
		assert_eq!(p.disc, Some(1));
		assert_eq!(p.regions, vec![RegionTag::Usa]);
	}

	#[test]
	fn disc_with_total() {
		let p = parse_name("Final Fantasy VII (Disc 2 of 3)");
		assert_eq!(p.disc, Some(2));
	}

	#[test]
	fn side_a() {
		let p = parse_name("Maniac Mansion (Side A)");
		assert_eq!(p.disc, Some(1));
	}

	#[test]
	fn year_only() {
		let p = parse_name("Solitaire (1990)");
		assert_eq!(p.base, "Solitaire");
		assert_eq!(p.year, Some(1990));
	}

	#[test]
	fn nested_parens() {
		let p = parse_name("Game (Foo (Bar))");
		assert_eq!(p.base, "Game");
		assert_eq!(p.residual, vec!["Foo (Bar)"]);
	}

	#[test]
	fn square_brackets_to_residual() {
		let p = parse_name("Mario [b1]");
		assert_eq!(p.base, "Mario");
		assert_eq!(p.residual, vec!["b1"]);
	}

	#[test]
	fn unmatched_paren_keeps_in_base() {
		let p = parse_name("Game (USA");
		assert_eq!(p.base, "Game (USA");
		assert!(p.regions.is_empty());
	}

	#[test]
	fn unknown_paren_to_residual_not_to_base() {
		let p = parse_name("Game (Track 1)");
		assert_eq!(p.base, "Game");
		assert_eq!(p.residual, vec!["Track 1"]);
	}

	#[test]
	fn full_no_intro_name() {
		let p = parse_name("Pokemon - Red Version (USA, Europe) (En,Fr,De,Es,It) (Rev A)");
		assert_eq!(p.base, "Pokemon - Red Version");
		assert_eq!(p.regions, vec![RegionTag::Usa, RegionTag::Europe]);
		assert_eq!(
			p.languages,
			vec![Lang::En, Lang::Fr, Lang::De, Lang::Es, Lang::It]
		);
		assert_eq!(p.revision.as_deref(), Some("Rev A"));
	}

	#[test]
	fn region_short_codes() {
		let p = parse_name("Tetris (J)");
		assert_eq!(p.regions, vec![RegionTag::Japan]);
	}

	#[test]
	fn language_with_locale() {
		let p = parse_name("Game (En-US,Fr-CA)");
		assert_eq!(p.languages, vec![Lang::En, Lang::Fr]);
	}

	#[test]
	fn lowercase_region_falls_through_to_residual() {
		let p = parse_name("Game (usa)");
		assert!(p.regions.is_empty());
		assert_eq!(p.residual, vec!["usa"]);
	}

	#[test]
	fn out_of_range_year_to_residual() {
		let p = parse_name("Future Game (3000)");
		assert!(p.year.is_none());
		assert_eq!(p.residual, vec!["3000"]);
	}

	#[test]
	fn ss_codes_japan() {
		assert_eq!(RegionTag::Japan.ss_codes(), &["jp"]);
	}

	#[test]
	fn ss_codes_asia_multi() {
		assert_eq!(RegionTag::Asia.ss_codes(), &["asi", "cn", "ko"]);
	}

	#[test]
	fn lb_codes_usa_alternates() {
		assert_eq!(
			RegionTag::Usa.lb_codes(),
			&["United States", "USA", "North America"]
		);
	}
}
