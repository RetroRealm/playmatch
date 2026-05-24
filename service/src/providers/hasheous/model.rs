use serde::{Deserialize, Serialize};

/// Hasheous expects mixed-case JSON keys (`mD5`, `shA1`, `shA256`) per the
/// public swagger contract. Do not change these or the request body silently
/// becomes a 404.
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct HashLookupRequest {
	#[serde(rename = "mD5", skip_serializing_if = "Option::is_none")]
	pub md5: Option<String>,
	#[serde(rename = "shA1", skip_serializing_if = "Option::is_none")]
	pub sha1: Option<String>,
	#[serde(rename = "shA256", skip_serializing_if = "Option::is_none")]
	pub sha256: Option<String>,
	#[serde(rename = "crc", skip_serializing_if = "Option::is_none")]
	pub crc: Option<String>,
}

impl HashLookupRequest {
	pub fn is_empty(&self) -> bool {
		self.md5.is_none() && self.sha1.is_none() && self.sha256.is_none() && self.crc.is_none()
	}
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HashLookupResponse {
	pub id: i64,
	#[serde(default)]
	pub name: Option<String>,
	#[serde(default)]
	pub platform: Option<MiniDataObjectItem>,
	#[serde(default)]
	pub publisher: Option<MiniDataObjectItem>,
	#[serde(default)]
	pub signature: Option<SignatureLookupItem>,
	#[serde(default)]
	pub metadata: Option<Vec<DataObjectItemMetadata>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MiniDataObjectItem {
	#[serde(default)]
	pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureLookupItem {
	#[serde(default)]
	pub game: Option<HasheousGame>,
	#[serde(default)]
	pub rom: Option<HasheousRom>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HasheousGame {
	#[serde(default)]
	pub id: Option<String>,
	#[serde(default)]
	pub name: Option<String>,
	#[serde(default)]
	pub year: Option<String>,
	#[serde(default)]
	pub publisher: Option<String>,
	#[serde(default)]
	pub system: Option<String>,
	#[serde(default)]
	pub roms: Option<Vec<HasheousRom>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HasheousRom {
	#[serde(default)]
	pub md5: Option<String>,
	#[serde(default)]
	pub sha1: Option<String>,
	#[serde(default)]
	pub sha256: Option<String>,
	#[serde(default)]
	pub crc: Option<String>,
	#[serde(default)]
	pub name: Option<String>,
	#[serde(default)]
	pub size: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataObjectItemMetadata {
	#[serde(default)]
	pub id: Option<String>,
	#[serde(default)]
	pub immutable_id: Option<String>,
	#[serde(default)]
	pub source: Option<String>,
	#[serde(default)]
	pub link: Option<String>,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn hash_lookup_request_serialises_with_mixed_case_keys() {
		let req = HashLookupRequest {
			md5: Some("a".repeat(32)),
			sha1: Some("b".repeat(40)),
			sha256: Some("c".repeat(64)),
			crc: Some("12345678".into()),
		};
		let json = serde_json::to_string(&req).expect("serialize");
		assert!(json.contains("\"mD5\":"), "got: {json}");
		assert!(json.contains("\"shA1\":"), "got: {json}");
		assert!(json.contains("\"shA256\":"), "got: {json}");
		assert!(json.contains("\"crc\":"), "got: {json}");
	}

	#[test]
	fn hash_lookup_request_skips_absent_hashes() {
		let req = HashLookupRequest {
			md5: Some("a".repeat(32)),
			..Default::default()
		};
		let json = serde_json::to_string(&req).expect("serialize");
		assert!(json.contains("\"mD5\":"));
		assert!(!json.contains("shA1"));
		assert!(!json.contains("shA256"));
		assert!(!json.contains("\"crc\":"));
	}

	#[test]
	fn hash_lookup_request_is_empty_when_all_none() {
		assert!(HashLookupRequest::default().is_empty());
		assert!(
			!HashLookupRequest {
				md5: Some("abc".into()),
				..Default::default()
			}
			.is_empty()
		);
	}

	#[test]
	fn hash_lookup_response_parses_minimal_payload() {
		let raw = r#"{"id": 42, "name": "Some Game"}"#;
		let parsed: HashLookupResponse = serde_json::from_str(raw).expect("deserialize");
		assert_eq!(parsed.id, 42);
		assert_eq!(parsed.name.as_deref(), Some("Some Game"));
		assert!(parsed.signature.is_none());
		assert!(parsed.metadata.is_none());
	}

	#[test]
	fn hash_lookup_response_parses_full_signature_payload() {
		let raw = r#"{
			"id": 7,
			"name": "Alpha",
			"platform": {"name": "Mega Drive"},
			"publisher": {"name": "Sega"},
			"signature": {
				"game": {"id": "abc", "name": "Alpha", "year": "1990"},
				"rom": {"sha1": "aa", "md5": "bb", "crc": "cc"}
			}
		}"#;
		let parsed: HashLookupResponse = serde_json::from_str(raw).expect("deserialize");
		assert_eq!(parsed.id, 7);
		assert_eq!(
			parsed.platform.as_ref().and_then(|p| p.name.as_deref()),
			Some("Mega Drive")
		);
		let sig = parsed.signature.expect("signature present");
		assert_eq!(sig.game.unwrap().year.as_deref(), Some("1990"));
		let rom = sig.rom.expect("rom present");
		assert_eq!(rom.sha1.as_deref(), Some("aa"));
		assert_eq!(rom.crc.as_deref(), Some("cc"));
	}
}
