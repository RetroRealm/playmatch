use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};
use utoipa::ToSchema;

/// Region codes ScreenScraper can return on `noms` entries, in the order we
/// prefer when picking a primary display name. Anything not in the list is
/// considered an alternative-name candidate at lower priority.
pub const REGION_PRIORITY: &[&str] = &[
	"wor", "us", "eu", "ss", "au", "jp", "br", "asi", "cn", "ko", "de", "fr",
];

// SsHeader, SsUser and SsServeurs are deliberately deserialize-only: the
// `ssuser` block carries account-private fields (numid, niveau, last visit,
// quota counters) and must never reach a response body or OpenAPI schema.
// Keep them off `Serialize` and `ToSchema` so future refactors cannot leak
// them by accident.
//
// ScreenScraper inconsistently returns numeric fields as either strings
// (`"1"`) or integers (`1`) across endpoints and over time. Every field
// historically declared `Option<String>` here uses the flexible
// deserializer below so the parse does not break when SS flips a field
// type mid-deploy.

#[derive(Debug, Clone, Deserialize)]
pub struct SsHeader {
	#[serde(default, deserialize_with = "de_flexible_string")]
	pub success: String,
	#[serde(default, deserialize_with = "de_opt_flexible_string")]
	pub error: Option<String>,
	#[serde(
		rename = "APIversion",
		default,
		deserialize_with = "de_opt_flexible_string"
	)]
	pub api_version: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SsUser {
	#[serde(default, deserialize_with = "de_opt_flexible_string")]
	pub requeststoday: Option<String>,
	#[serde(default, deserialize_with = "de_opt_flexible_string")]
	pub maxrequestsperday: Option<String>,
	#[serde(default, deserialize_with = "de_opt_flexible_string")]
	pub maxrequestspermin: Option<String>,
	#[serde(default, deserialize_with = "de_opt_flexible_string")]
	pub maxthreads: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SsServeurs {
	#[serde(default, deserialize_with = "de_opt_flexible_string")]
	pub closefornomember: Option<String>,
	#[serde(default, deserialize_with = "de_opt_flexible_string")]
	pub closeforleecher: Option<String>,
}

/// Top-level envelope. `response` is `Option` because incident responses may
/// include only a `header` with `success: "false"`.
#[derive(Debug, Clone, Deserialize)]
pub struct SsEnvelope<T> {
	#[serde(default)]
	pub header: Option<SsHeader>,
	pub response: Option<SsResponse<T>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SsResponse<T> {
	#[serde(default)]
	pub ssuser: Option<SsUser>,
	#[serde(default)]
	pub serveurs: Option<SsServeurs>,
	#[serde(flatten)]
	pub payload: T,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JeuPayload {
	pub jeu: SsGame,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JeuxPayload {
	#[serde(default)]
	pub jeux: Vec<SsGame>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SystemesPayload {
	#[serde(default)]
	pub systemes: Vec<SsSystem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SsSystem {
	#[serde(deserialize_with = "de_string_i32")]
	pub id: i32,
	#[serde(default)]
	pub noms: SsSystemNames,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct SsSystemNames {
	#[serde(default)]
	pub nom_eu: Option<String>,
	#[serde(default)]
	pub nom_us: Option<String>,
	#[serde(default)]
	pub nom_jp: Option<String>,
	#[serde(default)]
	pub noms_commun: Option<String>,
	#[serde(default)]
	pub nom_recalbox: Option<String>,
	#[serde(default)]
	pub nom_retropie: Option<String>,
	#[serde(default)]
	pub nom_launchbox: Option<String>,
	#[serde(default)]
	pub nom_hyperspin: Option<String>,
}

impl SsSystem {
	/// Iterates every non-empty name field in priority order. Used both for
	/// matching DB platforms to SS systems and for picking a display name.
	pub fn iter_names(&self) -> impl Iterator<Item = &str> {
		[
			self.noms.noms_commun.as_deref(),
			self.noms.nom_eu.as_deref(),
			self.noms.nom_us.as_deref(),
			self.noms.nom_jp.as_deref(),
			self.noms.nom_recalbox.as_deref(),
			self.noms.nom_retropie.as_deref(),
			self.noms.nom_launchbox.as_deref(),
			self.noms.nom_hyperspin.as_deref(),
		]
		.into_iter()
		.flatten()
		.filter(|s| !s.is_empty())
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SsLocalizedName {
	pub region: String,
	pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SsRom {
	#[serde(default, deserialize_with = "de_opt_flexible_string")]
	pub romfilename: Option<String>,
	#[serde(default, deserialize_with = "de_opt_flexible_string")]
	pub rommd5: Option<String>,
	#[serde(default, deserialize_with = "de_opt_flexible_string")]
	pub romsha1: Option<String>,
	#[serde(default, deserialize_with = "de_opt_flexible_string")]
	pub romcrc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SsGame {
	#[serde(deserialize_with = "de_string_i64")]
	pub id: i64,
	#[serde(default)]
	pub noms: Vec<SsLocalizedName>,
	#[serde(default)]
	pub roms: Option<Vec<SsRom>>,
	#[serde(default)]
	pub editeur: Option<SsEntityRef>,
	#[serde(default)]
	pub developpeur: Option<SsEntityRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SsEntityRef {
	#[serde(default, deserialize_with = "de_opt_flexible_string")]
	pub id: Option<String>,
	#[serde(default, deserialize_with = "de_opt_flexible_string")]
	pub text: Option<String>,
}

impl SsGame {
	/// Iterate every candidate name across regions, preferring the
	/// `REGION_PRIORITY` order, then any remaining region. Used by the matcher
	/// so a game returned with only a `jp` name still produces a hit.
	pub fn iter_candidate_names(&self) -> impl Iterator<Item = &str> {
		let priority: Vec<&str> = REGION_PRIORITY
			.iter()
			.filter_map(|region| {
				self.noms
					.iter()
					.find(|n| n.region.eq_ignore_ascii_case(region))
					.map(|n| n.text.as_str())
			})
			.collect();
		let extras: Vec<&str> = self
			.noms
			.iter()
			.filter(|n| {
				!REGION_PRIORITY
					.iter()
					.any(|r| n.region.eq_ignore_ascii_case(r))
			})
			.map(|n| n.text.as_str())
			.collect();
		priority.into_iter().chain(extras)
	}
}

fn de_string_i32<'de, D: Deserializer<'de>>(d: D) -> Result<i32, D::Error> {
	let s = de_flexible_string(d)?;
	s.parse::<i32>().map_err(D::Error::custom)
}

fn de_string_i64<'de, D: Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
	let s = de_flexible_string(d)?;
	s.parse::<i64>().map_err(D::Error::custom)
}

#[derive(Deserialize)]
#[serde(untagged)]
enum FlexibleScalar {
	Str(String),
	Int(i64),
	UInt(u64),
	Float(f64),
	Bool(bool),
}

impl FlexibleScalar {
	fn into_string(self) -> String {
		match self {
			Self::Str(s) => s,
			Self::Int(i) => i.to_string(),
			Self::UInt(u) => u.to_string(),
			Self::Float(f) => f.to_string(),
			Self::Bool(b) => b.to_string(),
		}
	}
}

fn de_flexible_string<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
	Ok(FlexibleScalar::deserialize(d)?.into_string())
}

fn de_opt_flexible_string<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
	Ok(Option::<FlexibleScalar>::deserialize(d)?.map(FlexibleScalar::into_string))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn de_string_i32_parses_numeric_string() {
		let json = r#""12345""#;
		let v: i32 = serde_json::from_str::<String>(json)
			.unwrap()
			.parse()
			.unwrap();
		assert_eq!(v, 12345);
	}

	#[test]
	fn ss_envelope_jeu_parses_minimal_payload() {
		let body = r#"{
			"header": {"success": "true"},
			"response": {
				"jeu": {
					"id": "42",
					"noms": [
						{"region": "us", "text": "Sonic"},
						{"region": "jp", "text": "ソニック"}
					]
				}
			}
		}"#;
		let env: SsEnvelope<JeuPayload> = serde_json::from_str(body).unwrap();
		let jeu = env.response.unwrap().payload.jeu;
		assert_eq!(jeu.id, 42);
		assert_eq!(jeu.noms.len(), 2);
	}

	#[test]
	fn iter_candidate_names_prioritises_known_regions_then_extras() {
		let game = SsGame {
			id: 1,
			noms: vec![
				SsLocalizedName {
					region: "ko".into(),
					text: "Korean".into(),
				},
				SsLocalizedName {
					region: "us".into(),
					text: "American".into(),
				},
				SsLocalizedName {
					region: "xy".into(),
					text: "Unknown".into(),
				},
				SsLocalizedName {
					region: "wor".into(),
					text: "World".into(),
				},
			],
			roms: None,
			editeur: None,
			developpeur: None,
		};

		let names: Vec<&str> = game.iter_candidate_names().collect();
		assert_eq!(names[0], "World");
		assert_eq!(names[1], "American");
		assert_eq!(names[2], "Korean");
		assert_eq!(names.last().copied(), Some("Unknown"));
	}

	#[test]
	fn ss_system_iter_names_skips_empty_and_missing() {
		let system = SsSystem {
			id: 1,
			noms: SsSystemNames {
				nom_eu: Some("".into()),
				nom_us: Some("Super NES".into()),
				noms_commun: Some("Super Nintendo Entertainment System".into()),
				..Default::default()
			},
		};

		let names: Vec<&str> = system.iter_names().collect();
		assert_eq!(names[0], "Super Nintendo Entertainment System");
		assert_eq!(names[1], "Super NES");
	}
}
