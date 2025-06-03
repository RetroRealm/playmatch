use actix_governor::{KeyExtractor, SimpleKeyExtractionError};
use actix_web::dev::ServiceRequest;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct ReverProxyExtractor;

impl KeyExtractor for ReverProxyExtractor {
	type Key = IpAddr;
	type KeyExtractionError = SimpleKeyExtractionError<&'static str>;

	fn extract(&self, req: &ServiceRequest) -> Result<Self::Key, Self::KeyExtractionError> {
		let mut ip = req
			.connection_info()
			.realip_remote_addr()
			.map(IpAddr::from_str)
			.ok_or_else(|| {
				SimpleKeyExtractionError::new("Could not extract peer IP address from request")
			})?
			.map_err(|_| {
				SimpleKeyExtractionError::new("Could not extract peer IP address from request")
			})?;

		// customers often get their own /56 prefix, apply rate-limiting per prefix instead of per
		// address for IPv6
		if let IpAddr::V6(ipv6) = ip {
			let mut octets = ipv6.octets();
			octets[7..16].fill(0);
			ip = IpAddr::V6(octets.into());
		}
		Ok(ip)
	}
}
