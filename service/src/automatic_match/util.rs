/// Convert a Roman numeral (up to 3999) to an integer.
pub fn roman_to_int(roman: &str) -> Option<u32> {
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
