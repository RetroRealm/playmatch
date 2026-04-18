use actix_web::Error;
use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::middleware::Next;
use service::metrics::record_user_agent;

pub async fn user_agent_metric<B: MessageBody>(
	req: ServiceRequest,
	next: Next<B>,
) -> Result<ServiceResponse<B>, Error> {
	let ua = req
		.headers()
		.get("User-Agent")
		.and_then(|h| h.to_str().ok())
		.unwrap_or("");
	let (product, version) = classify_user_agent(ua);
	record_user_agent(product, &version);

	next.call(req).await
}

fn classify_user_agent(ua: &str) -> (&'static str, String) {
	if ua.is_empty() {
		return ("none", String::new());
	}

	if let Some(rest) = ua.strip_prefix("RomM/") {
		let raw = rest
			.split(|c: char| c.is_whitespace() || c == ';' || c == ',')
			.next()
			.unwrap_or("");
		let version = if is_safe_version(raw) {
			raw.to_string()
		} else {
			"unknown".to_string()
		};
		return ("romm", version);
	}

	let ua_lower = ua.to_ascii_lowercase();
	if ua_lower.contains("bot")
		|| ua_lower.contains("crawler")
		|| ua_lower.contains("spider")
		|| ua_lower.contains("scraper")
	{
		return ("bot", String::new());
	}

	if ua.starts_with("curl/") {
		return ("curl", String::new());
	}

	if ua.starts_with("Mozilla/") {
		if ua.contains("Firefox/") {
			return ("browser", "firefox".to_string());
		}
		if ua.contains("Edg/") {
			return ("browser", "edge".to_string());
		}
		if ua.contains("Chrome/") {
			return ("browser", "chrome".to_string());
		}
		if ua.contains("Safari/") {
			return ("browser", "safari".to_string());
		}
		return ("browser", "other".to_string());
	}

	("other", String::new())
}

fn is_safe_version(v: &str) -> bool {
	!v.is_empty()
		&& v.len() <= 32
		&& v.chars()
			.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+' | '_'))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn empty_ua_classifies_as_none() {
		assert_eq!(classify_user_agent(""), ("none", String::new()));
	}

	#[test]
	fn romm_version_is_extracted() {
		assert_eq!(
			classify_user_agent("RomM/3.5.0"),
			("romm", "3.5.0".to_string())
		);
		assert_eq!(
			classify_user_agent("RomM/3.5.0-beta.1"),
			("romm", "3.5.0-beta.1".to_string())
		);
	}

	#[test]
	fn romm_version_with_suffix_is_cleaned() {
		assert_eq!(
			classify_user_agent("RomM/3.5.0 (Python/3.11)"),
			("romm", "3.5.0".to_string())
		);
	}

	#[test]
	fn romm_garbage_version_buckets_to_unknown() {
		let garbage = "RomM/".to_string() + &"x".repeat(100);
		assert_eq!(
			classify_user_agent(&garbage),
			("romm", "unknown".to_string())
		);
		assert_eq!(
			classify_user_agent("RomM/<script>"),
			("romm", "unknown".to_string())
		);
	}

	#[test]
	fn bot_user_agents_are_bucketed() {
		assert_eq!(
			classify_user_agent(
				"Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)"
			),
			("bot", String::new())
		);
		assert_eq!(
			classify_user_agent("SemrushBot/7~bl"),
			("bot", String::new())
		);
	}

	#[test]
	fn curl_is_bucketed_without_version() {
		assert_eq!(classify_user_agent("curl/8.4.0"), ("curl", String::new()));
	}

	#[test]
	fn browsers_are_bucketed() {
		assert_eq!(
			classify_user_agent(
				"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
			),
			("browser", "chrome".to_string())
		);
		assert_eq!(
			classify_user_agent(
				"Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/117.0"
			),
			("browser", "firefox".to_string())
		);
	}

	#[test]
	fn unknown_ua_buckets_to_other() {
		assert_eq!(
			classify_user_agent("SomeCustomClient 1.0"),
			("other", String::new())
		);
	}
}
