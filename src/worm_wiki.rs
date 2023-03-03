use std::sync::Arc;

use crate::html;
use crate::layout::{Layout, LayoutComponent, LayoutParser};
use crate::parser::Parser;
use crate::requester::SimpleRequest;
use crate::spider::RequestFilter;
use reqwest::header::HeaderMap;
use reqwest::{Method, Url};
use scraper::{Html, Selector};

#[derive(Clone)]
pub struct WormWikiListOfCharacters {
    layout_parser: LayoutParser<Html, Extractions>,
}

impl WormWikiListOfCharacters {
    pub fn new() -> WormWikiListOfCharacters {
        WormWikiListOfCharacters {
            layout_parser: LayoutParser {
                layouts: Arc::new(vec![Layout {
                    components: vec![Box::new(ArticleLinksComponent {})],
                }]),
            },
        }
    }
}

pub fn initial() -> Vec<SimpleRequest> {
    vec![SimpleRequest {
        method: Method::GET,
        url: Url::parse("https://worm.fandom.com/wiki/Worm_Wiki").unwrap(),
        headers: HeaderMap::new(),
        body: None,
    }]
}

pub struct WormRequestFilter;

impl RequestFilter for WormRequestFilter {
    fn is_valid(&self, request: &SimpleRequest) -> bool {
        let u = request.url.as_str();
        u.starts_with("https://worm.fandom.com/wiki")
    }
}

impl Parser for WormWikiListOfCharacters {
    async fn parse(self, request: &SimpleRequest, page: &str) -> Vec<SimpleRequest> {
        self.layout_parser
            .parse(request, page, html::parse, router)
            .await
    }
}

async fn router(extractions: Vec<Extractions>) -> Vec<SimpleRequest> {
    extractions
        .into_iter()
        .map(|e| match e {
            Extractions::URL(u) => SimpleRequest {
                method: Method::GET,
                url: u,
                headers: HeaderMap::new(),
                body: None,
            },
        })
        .collect()
}

#[derive(Clone)]
enum Extractions {
    URL(Url),
}

struct ArticleLinksComponent {}
impl LayoutComponent<Html, Extractions> for ArticleLinksComponent {
    fn matches(&self, content: &Html) -> bool {
        let selector = Selector::parse("a").unwrap();
        content.select(&selector).next().is_some()
    }

    fn extract(&self, request: &SimpleRequest, content: &Html) -> Vec<Extractions> {
        let selector = Selector::parse("a").unwrap();
        content
            .select(&selector)
            .into_iter()
            .flat_map(|s| s.value().attr("href"))
            .flat_map(|u| {
                if u.starts_with("/") {
                    Some(Extractions::URL(request.url.join(u).unwrap()))
                } else if u.starts_with("http") {
                    Some(Extractions::URL(Url::parse(u).unwrap()))
                } else {
                    None
                }
            })
            .collect()
    }
}
