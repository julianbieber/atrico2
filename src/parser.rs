use std::future::Future;

use reqwest::Request;


pub trait Parser: Send + 'static {
    fn parse(self, page: &str) -> impl Future<Output = Vec<Request>> + Send + '_ ;
}
