use actix_web::http::header::{CacheControl, CacheDirective};
use actix_web::HttpResponseBuilder;

pub trait InsertCacheHeaders {
    fn set_public_cacheable(&mut self, max_age: u32);
}

impl InsertCacheHeaders for HttpResponseBuilder {
    fn set_public_cacheable(&mut self, max_age: u32) {
        self.insert_header(CacheControl(vec![
            CacheDirective::Public,
            CacheDirective::MaxAge(max_age),
        ]));
    }
}
