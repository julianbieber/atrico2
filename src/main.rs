#![feature(return_position_impl_trait_in_trait)]
#![feature(async_fn_in_trait)]

use requester::SimpleRequest;
use reqwest::{header::HeaderMap, Method, Url};
use worm_wiki::{WormRequestFilter, WormWikiListOfCharacters};

mod html;
mod layout;
mod parser;
mod requester;
mod spider;
mod worm_wiki;

#[tokio::main]
async fn main() {
    spider::Spider::run(
        worm_wiki::initial(),
        WormWikiListOfCharacters::new(),
        WormRequestFilter,
        "page_cache".into(),
    )
    .await
}
