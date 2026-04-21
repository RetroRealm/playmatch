use actix_governor::{KeyExtractor, SimpleKeyExtractionError};
use actix_web::HttpRequest;
use actix_web::dev::ServiceRequest;
use serde::{Deserialize, Serialize};
use std::env;
use std::net::IpAddr;
use std::str::FromStr;

/// Set to `false` in local development where no Cloudflare sits in front.
const TRUST_CF_CONNECTING_IP_ENV: &str = "TRUST_CF_CONNECTING_IP";

fn trust_cf_connecting_ip() -> bool {
	env::var(TRUST_CF_CONNECTING_IP_ENV)
		.map(|v| !matches!(v.to_ascii_lowercase().as_str(), "0" | "false" | "no"))
		.unwrap_or(true)
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct ReverProxyExtractor;

impl KeyExtractor for ReverProxyExtractor {
	type Key = IpAddr;
	type KeyExtractionError = SimpleKeyExtractionError<&'static str>;

	fn extract(&self, req: &ServiceRequest) -> Result<Self::Key, Self::KeyExtractionError> {
		let mut ip = extract_client_ip(req)?;

		// Collapse IPv6 to the /56 prefix customers typically share.
		if let IpAddr::V6(ipv6) = ip {
			let mut octets = ipv6.octets();
			octets[7..16].fill(0);
			ip = IpAddr::V6(octets.into());
		}
		Ok(ip)
	}
}

fn extract_client_ip(
	req: &ServiceRequest,
) -> Result<IpAddr, SimpleKeyExtractionError<&'static str>> {
	if trust_cf_connecting_ip()
		&& let Some(header) = req.headers().get("CF-Connecting-IP")
		&& let Ok(value) = header.to_str()
		&& let Ok(ip) = IpAddr::from_str(value.trim())
	{
		return Ok(ip);
	}

	req.peer_addr().map(|addr| addr.ip()).ok_or_else(|| {
		SimpleKeyExtractionError::new("Could not extract peer IP address from request")
	})
}

/// Mirrors [`ReverProxyExtractor`] for handlers that only have an [`HttpRequest`].
/// Keep in sync with [`extract_client_ip`] above.
pub fn client_ip_from_http_request(req: &HttpRequest) -> Option<IpAddr> {
	if trust_cf_connecting_ip()
		&& let Some(header) = req.headers().get("CF-Connecting-IP")
		&& let Ok(value) = header.to_str()
		&& let Ok(ip) = IpAddr::from_str(value.trim())
	{
		return Some(ip);
	}

	req.peer_addr().map(|addr| addr.ip())
}

#[cfg(test)]
mod tests {
	use super::*;
	use actix_web::test::TestRequest;

	// The TRUST_CF_CONNECTING_IP flag is not covered here because env vars are
	// process-global and racy under parallel cargo test.

	#[test]
	fn ipv4_cf_connecting_ip_is_used() {
		let req = TestRequest::default()
			.insert_header(("CF-Connecting-IP", "203.0.113.7"))
			.peer_addr("127.0.0.1:0".parse().unwrap())
			.to_srv_request();
		let ip = ReverProxyExtractor.extract(&req).unwrap();
		assert_eq!(ip, IpAddr::from_str("203.0.113.7").unwrap());
	}

	#[test]
	fn ipv6_cf_connecting_ip_is_masked_to_56() {
		let req = TestRequest::default()
			.insert_header(("CF-Connecting-IP", "2001:db8:1234:5678:9abc:def0:1234:5678"))
			.peer_addr("[::1]:0".parse().unwrap())
			.to_srv_request();
		let ip = ReverProxyExtractor.extract(&req).unwrap();
		assert_eq!(ip, IpAddr::from_str("2001:db8:1234:5600:0:0:0:0").unwrap());
	}

	#[test]
	fn missing_header_falls_back_to_peer_addr() {
		let req = TestRequest::default()
			.peer_addr("198.51.100.9:1234".parse().unwrap())
			.to_srv_request();
		let ip = ReverProxyExtractor.extract(&req).unwrap();
		assert_eq!(ip, IpAddr::from_str("198.51.100.9").unwrap());
	}

	#[test]
	fn malformed_header_falls_back_to_peer_addr() {
		let req = TestRequest::default()
			.insert_header(("CF-Connecting-IP", "not-an-ip"))
			.peer_addr("198.51.100.10:0".parse().unwrap())
			.to_srv_request();
		let ip = ReverProxyExtractor.extract(&req).unwrap();
		assert_eq!(ip, IpAddr::from_str("198.51.100.10").unwrap());
	}
}
