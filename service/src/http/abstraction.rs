use reqwest::{IntoUrl, RequestBuilder};

pub static mut USER_AGENT: String = String::new();

pub trait RequestClientExt {
	fn get_default_user_agent<U: IntoUrl>(&self, url: U) -> RequestBuilder;
}

#[allow(static_mut_refs)]
impl RequestClientExt for reqwest::Client {
	fn get_default_user_agent<U: IntoUrl>(&self, url: U) -> RequestBuilder {
		// Safety: USER_AGENT is only mutated in the main function
		unsafe { self.get(url).header("User-Agent", &USER_AGENT) }
	}
}
