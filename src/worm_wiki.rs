use std::sync::Arc;

use crate::html;
use crate::layout::{Layout, LayoutComponent, LayoutParser};
use crate::parser::Parser;
use crate::requester::SimpleRequest;
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

impl Parser for WormWikiListOfCharacters {
    async fn parse(self, page: &str) -> Vec<SimpleRequest> {
        self.layout_parser.parse(page, html::parse, router).await
    }
}

async fn router(extractions: Vec<Extractions>) -> Vec<SimpleRequest> {
    extractions
        .into_iter()
        .map(|e| match e {
            Extractions::URL(u) => SimpleRequest {
                method: Method::GET,
                url: Url::parse(&u).unwrap(),
                headers: HeaderMap::new(),
                body: None,
            },
        })
        .collect()
}

#[derive(Clone)]
enum Extractions {
    URL(String),
}

struct ArticleLinksComponent {}
impl LayoutComponent<Html, Extractions> for ArticleLinksComponent {
    fn matches(&self, content: &Html) -> bool {
        let selector = Selector::parse("a").unwrap();
        content.select(&selector).next().is_some()
    }

    fn extract(&self, content: &Html) -> Vec<Extractions> {
        let selector = Selector::parse("a").unwrap();
        content
            .select(&selector)
            .into_iter()
            .flat_map(|s| s.value().attr("href"))
            .map(|u| {
                if u.starts_with("https:") {
                    u.into()
                } else {
                    format!("https://worm.fandom.com{u}")
                }
            })
            .map(|u| Extractions::URL(u))
            .collect()
    }
}
