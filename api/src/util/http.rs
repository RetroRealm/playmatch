use actix_governor::{KeyExtractor, SimpleKeyExtractionError};
use actix_web::HttpRequest;
use actix_web::dev::ServiceRequest;
use serde::{Deserialize, Serialize};
use std::env;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::OnceLock;

/// Kill switch for `CF-Connecting-IP` trust. Set to `false`/`0`/`no`/`off` to
/// never honor the header regardless of the peer. Default on, but trust is still
/// gated on the peer being a configured trusted proxy (see [`TRUSTED_PROXY_CIDRS_ENV`]).
const TRUST_CF_CONNECTING_IP_ENV: &str = "TRUST_CF_CONNECTING_IP";

/// Comma-separated CIDR allowlist of the proxies permitted to set
/// `CF-Connecting-IP`. When unset it defaults to Cloudflare's published ranges
/// plus the private/loopback ranges (see [`PRIVATE_CIDRS`]), so both a
/// Cloudflare-fronted deployment and an app sitting behind a reverse proxy on a
/// private network (Traefik/nginx/ingress, Docker/compose) work with no
/// configuration, while a deployment reached directly over the public internet
/// still cannot be spoofed. Setting this REPLACES the defaults: give it your own
/// proxy's addresses to lock trust down, or an empty value to trust nothing.
const TRUSTED_PROXY_CIDRS_ENV: &str = "TRUSTED_PROXY_CIDRS";

/// Cloudflare's published edge ranges (https://www.cloudflare.com/ips/). Used as
/// the default trusted-proxy set so the common Cloudflare-fronted deployment needs
/// no configuration. Refresh if Cloudflare publishes new ranges, or override via
/// `TRUSTED_PROXY_CIDRS`.
const CLOUDFLARE_CIDRS: &[&str] = &[
	"173.245.48.0/20",
	"103.21.244.0/22",
	"103.22.200.0/22",
	"103.31.4.0/22",
	"141.101.64.0/18",
	"108.162.192.0/18",
	"190.93.240.0/20",
	"188.114.96.0/20",
	"197.234.240.0/22",
	"198.41.128.0/17",
	"162.158.0.0/15",
	"104.16.0.0/13",
	"104.24.0.0/14",
	"172.64.0.0/13",
	"131.0.72.0/22",
	"2400:cb00::/32",
	"2606:4700::/32",
	"2803:f800::/32",
	"2405:b500::/32",
	"2405:8100::/32",
	"2a06:98c0::/29",
	"2c0f:f248::/32",
];

/// Private, loopback and link-local ranges (RFC 1918, RFC 4193 ULA, loopback,
/// link-local) trusted by default alongside Cloudflare's. A container's listening
/// port is only reachable over its private network, so a peer with one of these
/// addresses is an infrastructure hop (a reverse proxy such as Traefik or nginx,
/// an ingress, or the Docker/compose network) rather than a direct internet
/// client. Trusting them lets an app behind a private-network proxy honor
/// `CF-Connecting-IP` with no configuration. A genuine public client reaches the
/// app with a public peer address, which is not in this set, so it still cannot
/// spoof the header. Override `TRUSTED_PROXY_CIDRS` if the port is exposed to an
/// untrusted private network.
const PRIVATE_CIDRS: &[&str] = &[
	"10.0.0.0/8",
	"172.16.0.0/12",
	"192.168.0.0/16",
	"127.0.0.0/8",
	"169.254.0.0/16",
	"::1/128",
	"fc00::/7",
	"fe80::/10",
];

fn cf_trust_enabled() -> bool {
	env::var(TRUST_CF_CONNECTING_IP_ENV)
		.map(|v| {
			!matches!(
				v.to_ascii_lowercase().as_str(),
				"0" | "false" | "no" | "off"
			)
		})
		.unwrap_or(true)
}

/// The parsed trusted-proxy networks, resolved once from the environment (or the
/// Cloudflare + private-range defaults). Read once because the value is fixed for
/// the process lifetime and this runs on every request.
fn trusted_proxy_networks() -> &'static [(IpAddr, u8)] {
	static NETWORKS: OnceLock<Vec<(IpAddr, u8)>> = OnceLock::new();
	NETWORKS.get_or_init(|| {
		let configured = env::var(TRUSTED_PROXY_CIDRS_ENV).ok();
		let raw: Vec<String> = match configured {
			Some(value) => value
				.split(',')
				.map(str::trim)
				.filter(|c| !c.is_empty())
				.map(str::to_string)
				.collect(),
			None => CLOUDFLARE_CIDRS
				.iter()
				.chain(PRIVATE_CIDRS)
				.map(|s| s.to_string())
				.collect(),
		};
		raw.iter().filter_map(|c| parse_cidr(c)).collect()
	})
}

/// Parse a `network/prefix` CIDR. Rejects a prefix wider than the address family.
fn parse_cidr(cidr: &str) -> Option<(IpAddr, u8)> {
	let (net, bits) = cidr.split_once('/')?;
	let net: IpAddr = net.trim().parse().ok()?;
	let bits: u8 = bits.trim().parse().ok()?;
	let max = if net.is_ipv4() { 32 } else { 128 };
	(bits <= max).then_some((net, bits))
}

/// Whether `ip` falls inside the `network/bits` CIDR. Different address families
/// never match.
fn ip_in_cidr(ip: IpAddr, network: IpAddr, bits: u8) -> bool {
	match (ip, network) {
		(IpAddr::V4(ip), IpAddr::V4(net)) => octets_share_prefix(&ip.octets(), &net.octets(), bits),
		(IpAddr::V6(ip), IpAddr::V6(net)) => octets_share_prefix(&ip.octets(), &net.octets(), bits),
		_ => false,
	}
}

fn octets_share_prefix(a: &[u8], b: &[u8], bits: u8) -> bool {
	let whole = (bits / 8) as usize;
	let remainder = bits % 8;
	if a[..whole] != b[..whole] {
		return false;
	}
	if remainder == 0 {
		return true;
	}
	let mask = 0xffu8 << (8 - remainder);
	(a[whole] & mask) == (b[whole] & mask)
}

/// Trust the `CF-Connecting-IP` header only when the kill switch is on and the
/// immediate peer is a configured trusted proxy. A direct client cannot then
/// spoof the header to mint a fresh rate-limit bucket.
fn peer_is_trusted_proxy(peer: Option<IpAddr>) -> bool {
	if !cf_trust_enabled() {
		return false;
	}
	match peer {
		Some(ip) => trusted_proxy_networks()
			.iter()
			.any(|(net, bits)| ip_in_cidr(ip, *net, *bits)),
		None => false,
	}
}

fn forwarded_ip_if_trusted<'a>(
	peer: Option<IpAddr>,
	header: impl FnOnce() -> Option<&'a str>,
) -> Option<IpAddr> {
	if !peer_is_trusted_proxy(peer) {
		return None;
	}
	header().and_then(|value| IpAddr::from_str(value.trim()).ok())
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
	let peer = req.peer_addr().map(|addr| addr.ip());

	if let Some(ip) = forwarded_ip_if_trusted(peer, || {
		req.headers()
			.get("CF-Connecting-IP")
			.and_then(|h| h.to_str().ok())
	}) {
		return Ok(ip);
	}

	peer.ok_or_else(|| {
		SimpleKeyExtractionError::new("Could not extract peer IP address from request")
	})
}

/// Mirrors [`ReverProxyExtractor`] for handlers that only have an [`HttpRequest`].
/// Keep in sync with [`extract_client_ip`] above.
pub fn client_ip_from_http_request(req: &HttpRequest) -> Option<IpAddr> {
	let peer = req.peer_addr().map(|addr| addr.ip());

	forwarded_ip_if_trusted(peer, || {
		req.headers()
			.get("CF-Connecting-IP")
			.and_then(|h| h.to_str().ok())
	})
	.or(peer)
}

#[cfg(test)]
mod tests {
	use super::*;
	use actix_web::test::TestRequest;

	// A Cloudflare edge address (inside 173.245.48.0/20) and a plain client that
	// is not a trusted proxy.
	const CF_PEER: &str = "173.245.48.7:0";
	const DIRECT_PEER: &str = "198.51.100.9:1234";

	#[test]
	fn parse_and_match_cidr_ipv4() {
		let (net, bits) = parse_cidr("104.16.0.0/13").unwrap();
		assert!(ip_in_cidr("104.20.30.40".parse().unwrap(), net, bits));
		assert!(!ip_in_cidr("203.0.113.1".parse().unwrap(), net, bits));
	}

	#[test]
	fn parse_and_match_cidr_ipv6() {
		let (net, bits) = parse_cidr("2606:4700::/32").unwrap();
		assert!(ip_in_cidr("2606:4700:1::abcd".parse().unwrap(), net, bits));
		assert!(!ip_in_cidr("2001:db8::1".parse().unwrap(), net, bits));
	}

	#[test]
	fn every_embedded_cloudflare_range_parses() {
		for cidr in CLOUDFLARE_CIDRS {
			assert!(parse_cidr(cidr).is_some(), "{cidr} must parse");
		}
	}

	#[test]
	fn every_embedded_private_range_parses() {
		for cidr in PRIVATE_CIDRS {
			assert!(parse_cidr(cidr).is_some(), "{cidr} must parse");
		}
	}

	#[test]
	fn private_network_peers_are_trusted_by_default() {
		// A reverse proxy on a private network (Docker/compose, k8s, a LAN) is a
		// trusted hop out of the box, so a CF-Connecting-IP forwarded through it is
		// honored with no TRUSTED_PROXY_CIDRS configuration.
		for peer in ["172.18.0.5", "10.1.2.3", "192.168.0.10", "127.0.0.1"] {
			assert!(
				peer_is_trusted_proxy(Some(peer.parse().unwrap())),
				"{peer} (a private-network proxy) must be trusted by default"
			);
		}
		assert!(
			!peer_is_trusted_proxy(Some("198.51.100.9".parse().unwrap())),
			"a public direct client must not be trusted"
		);
	}

	#[test]
	fn cf_connecting_ip_is_used_from_a_private_proxy_peer() {
		// Cloudflare -> Traefik (on a Docker network) -> app: the peer is Traefik's
		// private address and CF-Connecting-IP still resolves the real client.
		let req = TestRequest::default()
			.insert_header(("CF-Connecting-IP", "203.0.113.7"))
			.peer_addr("172.18.0.5:0".parse().unwrap())
			.to_srv_request();
		let ip = ReverProxyExtractor.extract(&req).unwrap();
		assert_eq!(ip, IpAddr::from_str("203.0.113.7").unwrap());
	}

	#[test]
	fn cross_family_never_matches() {
		let (net, bits) = parse_cidr("104.16.0.0/13").unwrap();
		assert!(!ip_in_cidr("2606:4700::1".parse().unwrap(), net, bits));
	}

	#[test]
	fn cf_connecting_ip_is_used_from_a_trusted_peer() {
		let req = TestRequest::default()
			.insert_header(("CF-Connecting-IP", "203.0.113.7"))
			.peer_addr(CF_PEER.parse().unwrap())
			.to_srv_request();
		let ip = ReverProxyExtractor.extract(&req).unwrap();
		assert_eq!(ip, IpAddr::from_str("203.0.113.7").unwrap());
	}

	#[test]
	fn spoofed_cf_connecting_ip_from_a_direct_peer_is_ignored() {
		let req = TestRequest::default()
			.insert_header(("CF-Connecting-IP", "203.0.113.7"))
			.peer_addr(DIRECT_PEER.parse().unwrap())
			.to_srv_request();
		let ip = ReverProxyExtractor.extract(&req).unwrap();
		assert_eq!(
			ip,
			IpAddr::from_str("198.51.100.9").unwrap(),
			"a direct client must not be able to set its own rate-limit key"
		);
	}

	#[test]
	fn ipv6_result_is_masked_to_56() {
		let req = TestRequest::default()
			.insert_header(("CF-Connecting-IP", "2001:db8:1234:5678:9abc:def0:1234:5678"))
			.peer_addr(CF_PEER.parse().unwrap())
			.to_srv_request();
		let ip = ReverProxyExtractor.extract(&req).unwrap();
		assert_eq!(ip, IpAddr::from_str("2001:db8:1234:5600:0:0:0:0").unwrap());
	}

	#[test]
	fn missing_header_falls_back_to_peer_addr() {
		let req = TestRequest::default()
			.peer_addr(DIRECT_PEER.parse().unwrap())
			.to_srv_request();
		let ip = ReverProxyExtractor.extract(&req).unwrap();
		assert_eq!(ip, IpAddr::from_str("198.51.100.9").unwrap());
	}

	#[test]
	fn malformed_header_from_trusted_peer_falls_back_to_peer_addr() {
		let req = TestRequest::default()
			.insert_header(("CF-Connecting-IP", "not-an-ip"))
			.peer_addr(CF_PEER.parse().unwrap())
			.to_srv_request();
		let ip = ReverProxyExtractor.extract(&req).unwrap();
		assert_eq!(ip, IpAddr::from_str("173.245.48.7").unwrap());
	}
}
