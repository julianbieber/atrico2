use std::future::Future;

use crate::requester::SimpleRequest;

pub trait Parser: Send + 'static {
    fn parse<'a>(
        self,
        request: &'a SimpleRequest,
        page: &'a str,
    ) -> impl Future<Output = Vec<SimpleRequest>> + Send + 'a;
}
