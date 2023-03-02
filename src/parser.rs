use std::future::Future;

use crate::requester::SimpleRequest;

pub trait Parser: Send + 'static {
    fn parse(self, page: &str) -> impl Future<Output = Vec<SimpleRequest>> + Send + '_;
}
