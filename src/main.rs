#![feature(return_position_impl_trait_in_trait)]
#![feature(async_fn_in_trait)]

use parser::Parser;

mod spider;
mod parser;
mod requester;


#[tokio::main]
async fn main() {
    spider::Spider::run(Vec::new(), TODOParser {}).await
}


#[derive(Clone, Copy)]
struct TODOParser {}

impl Parser for TODOParser {
    async fn parse(self, page: &str) -> Vec<reqwest::Request>  {
        todo!()
    }
}
