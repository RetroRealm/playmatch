use serde::Serialize;

const CARD_NAME: &str = "io.github.retrorealm/playmatch";
const CARD_DESCRIPTION: &str = "Identifies game ROMs by hash and exposes the Playmatch catalogue of games, platforms, companies and signature groups.";
const CARD_TITLE: &str = "Playmatch";
const CARD_WEBSITE: &str = "https://github.com/RetroRealm/playmatch";

#[derive(Serialize)]
struct ServerCard<'a> {
	name: &'a str,
	description: &'a str,
	version: &'a str,
	title: &'a str,
	#[serde(rename = "websiteUrl")]
	website_url: &'a str,
	repository: Repository<'a>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	remotes: Vec<Remote>,
}

#[derive(Serialize)]
struct Repository<'a> {
	url: &'a str,
	source: &'a str,
}

#[derive(Serialize)]
struct Remote {
	#[serde(rename = "type")]
	transport: &'static str,
	url: String,
}

/// Render the MCP Server Card (SEP-2127) advertised at the `.well-known` paths.
/// `remotes` is emitted only when a public base URL is configured, so an
/// operator who has not set one still serves a valid core card.
pub fn server_card_json(public_base_url: Option<&str>) -> String {
	let remotes = public_base_url
		.map(|base| {
			vec![Remote {
				transport: "streamable-http",
				url: format!("{}/mcp", base.trim_end_matches('/')),
			}]
		})
		.unwrap_or_default();

	let card = ServerCard {
		name: CARD_NAME,
		description: CARD_DESCRIPTION,
		version: env!("CARGO_PKG_VERSION"),
		title: CARD_TITLE,
		website_url: CARD_WEBSITE,
		repository: Repository {
			url: CARD_WEBSITE,
			source: "github",
		},
		remotes,
	};

	serde_json::to_string_pretty(&card).expect("server card serializes")
}

#[cfg(test)]
mod tests {
	use super::server_card_json;
	use serde_json::Value;

	#[test]
	fn card_with_public_url_advertises_the_mcp_remote() {
		let card: Value = serde_json::from_str(&server_card_json(Some("https://h"))).unwrap();
		assert_eq!(card["name"], "io.github.retrorealm/playmatch");
		assert!(card.get("$schema").is_none());
		assert_eq!(card["version"], env!("CARGO_PKG_VERSION"));
		assert_eq!(card["title"], "Playmatch");
		assert_eq!(card["remotes"][0]["type"], "streamable-http");
		assert_eq!(card["remotes"][0]["url"], "https://h/mcp");
	}

	#[test]
	fn card_trims_trailing_slash_on_base_url() {
		let card: Value = serde_json::from_str(&server_card_json(Some("https://h/"))).unwrap();
		assert_eq!(card["remotes"][0]["url"], "https://h/mcp");
	}

	#[test]
	fn card_without_public_url_omits_remotes() {
		let card: Value = serde_json::from_str(&server_card_json(None)).unwrap();
		assert!(card.get("remotes").is_none());
		assert_eq!(card["name"], "io.github.retrorealm/playmatch");
	}
}
