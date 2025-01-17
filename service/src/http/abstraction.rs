use crate::constants::http::REQWEST_DEFAULT_USER_AGENT;
use futures_util::future;
use reqwest::{IntoUrl, Request, RequestBuilder, Response};
use tower::retry::Policy;

#[derive(Debug, Clone)]
pub struct RetryPolicy(pub usize);

impl<E> Policy<Request, Response, E> for RetryPolicy {
	type Future = future::Ready<()>;

	fn retry(&mut self, _: &mut Request, result: &mut Result<Response, E>) -> Option<Self::Future> {
		if self.0 == 0 {
			return None;
		}

		if result.is_err() {
			self.0 -= 1;
			Some(future::ready(()))
		} else if let Ok(res) = result {
			if res.status().is_server_error() {
				self.0 -= 1;
				Some(future::ready(()))
			} else {
				None
			}
		} else {
			None
		}
	}

	fn clone_request(&mut self, req: &Request) -> Option<Request> {
		req.try_clone()
	}
}

pub trait RequestClientExt {
	fn get_default_user_agent<U: IntoUrl>(&self, url: U) -> RequestBuilder;
}

#[allow(static_mut_refs)]
impl RequestClientExt for reqwest::Client {
	fn get_default_user_agent<U: IntoUrl>(&self, url: U) -> RequestBuilder {
		self.get(url)
			.header("User-Agent", REQWEST_DEFAULT_USER_AGENT.as_str())
	}
}
