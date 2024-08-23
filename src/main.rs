#![feature(return_position_impl_trait_in_trait)]
#![feature(async_closure)]
#![feature(async_fn_in_trait)]

use std::fs::File;

use tracing_subscriber::{filter::FilterFn, prelude::*};
use worm_wiki::{WormRequestFilter, WormWikiListOfCharacters};

mod html;
mod layout;
mod parser;
mod requester;
mod spider;
mod worm_wiki;

#[tokio::main]
async fn main() {
    setup_logging();
    spider::Spider::run(
        worm_wiki::initial(),
        WormWikiListOfCharacters::new("characters".into()),
        WormRequestFilter,
        "page_cache".into(),
    )
    .await
}

fn setup_logging() {
    let file = File::create("log.jsonl").unwrap();
    let json_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_writer(file)
        .with_filter(FilterFn::new(|meta| {
            let html5ever_path = meta.module_path().is_some_and(|p| p.contains("html5ever"));
            let html5ever_target = meta.target().contains("html5ever");
            let selector_target = meta.target().contains("selector");
            !(html5ever_path || html5ever_target || selector_target)
        }));
    tracing_subscriber::registry().with(json_layer).init();
}
