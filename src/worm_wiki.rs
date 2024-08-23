use std::path::PathBuf;
use std::sync::Arc;

use crate::html;
use crate::layout::{Layout, LayoutComponent, LayoutParser};
use crate::parser::Parser;
use crate::requester::SimpleRequest;
use crate::spider::RequestFilter;
use once_cell::sync::Lazy;
use reqwest::header::HeaderMap;
use reqwest::{Method, Url};
use scraper::{element_ref, ElementRef, Html, Selector};

#[derive(Clone)]
pub struct WormWikiListOfCharacters {
    layout_parser: LayoutParser<Html, Extractions>,
    cahracter_to_disk: CharacterToDisk,
}

#[derive(Clone)]
struct CharacterToDisk {
    base_dir: PathBuf,
}

impl WormWikiListOfCharacters {
    pub fn new(out: PathBuf) -> WormWikiListOfCharacters {
        WormWikiListOfCharacters {
            layout_parser: LayoutParser {
                layouts: Arc::new(vec![
                    Layout {
                        components: vec![Box::new(ArticleLinksComponent), Box::new(MainPageBanner)],
                    },
                    Layout {
                        components: vec![Box::new(ArticleLinksComponent), Box::new(StoryArticle)],
                    },
                    Layout {
                        components: vec![Box::new(ArticleLinksComponent), Box::new(ChapterSumary)],
                    },
                    Layout {
                        components: vec![Box::new(ArticleLinksComponent), Box::new(ArcSummary)],
                    },
                    Layout {
                        components: vec![Box::new(ArticleLinksComponent), Box::new(CategoryPage)],
                    },
                    Layout {
                        components: vec![
                            Box::new(ArticleLinksComponent),
                            Box::new(CharacterSheetComponent),
                        ],
                    },
                ]),
            },
            cahracter_to_disk: CharacterToDisk { base_dir: out },
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
        (self.layout_parser)
            .clone()
            .parse(request, page, html::parse, async move |e| {
                (self).router(e).await
            })
            .await
    }
}

impl WormWikiListOfCharacters {
    async fn router(&self, extractions: Vec<Extractions>) -> Vec<SimpleRequest> {
        extractions
            .into_iter()
            .flat_map(|e| match e {
                Extractions::URL(u) => Some(SimpleRequest {
                    method: Method::GET,
                    url: u,
                    headers: HeaderMap::new(),
                    body: None,
                }),
                Extractions::Character(names) => {
                    dbg!(names);
                    None
                }
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
enum Extractions {
    URL(Url),
    Character(Vec<String>),
}

#[derive(Debug)]
struct ArticleLinksComponent;
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
                    Some(request.url.join(u).unwrap())
                } else if u.starts_with("http") {
                    Some(Url::parse(u).unwrap())
                } else {
                    None
                }
            })
            .map(drop_get_parameters)
            .map(Extractions::URL)
            .collect()
    }
}

fn drop_get_parameters(mut url: Url) -> Url {
    url.set_query(None);
    url
}

#[derive(Debug)]
struct CharacterSheetComponent;
impl CharacterSheetComponent {
    const MAIN_NAME_SELECTOR: Lazy<Selector> =
        Lazy::new(|| Selector::parse("[data-source~=name]").unwrap());
    const ALIAS_SELECTOR: Lazy<Selector> =
        Lazy::new(|| Selector::parse(".pi-group [data-source~=alias] .pi-data-value").unwrap());

    fn extract_character(&self, section: ElementRef) -> Option<Extractions> {
        let main_name = get_text(
            section
                .select(&CharacterSheetComponent::MAIN_NAME_SELECTOR)
                .next()
                .unwrap(),
        )
        .pop()
        .unwrap();
        let mut names = vec![main_name];
        let aliases = section
            .select(&CharacterSheetComponent::ALIAS_SELECTOR)
            .map(|s| get_text(s))
            .next()?;
        names.extend_from_slice(&aliases);
        let names = clean_names(names);
        Some(Extractions::Character(names))
    }
}

fn get_text(element: ElementRef) -> Vec<String> {
    element.text().map(|t| t.into()).collect::<Vec<String>>()
}

fn clean_names(names: Vec<String>) -> Vec<String> {
    // assuming they are not nested
    let mut currently_in_brackets = false;
    let open_braces = ['(', '['];
    let closing_braces = [')', ']'];
    names
        .into_iter()
        .map(|name| {
            name.chars()
                .into_iter()
                .flat_map(|char| {
                    if open_braces.contains(&char) || currently_in_brackets {
                        currently_in_brackets = !closing_braces.contains(&char);
                        None
                    } else {
                        Some(char)
                    }
                })
                .collect()
        })
        .map(|name: String| name.trim_end().trim_start().into())
        .filter(|name: &String| !name.is_empty())
        .collect()
}

impl LayoutComponent<Html, Extractions> for CharacterSheetComponent {
    fn matches(&self, content: &Html) -> bool {
        let selector = Selector::parse(".portable-infobox").unwrap();
        content.select(&selector).next().is_some()
    }

    fn extract(&self, _request: &SimpleRequest, content: &Html) -> Vec<Extractions> {
        let selector = Selector::parse(".portable-infobox").unwrap();
        content
            .select(&selector)
            .into_iter()
            .flat_map(|s| self.extract_character(s))
            .collect()
    }
}

#[derive(Debug)]
struct MainPageBanner;
impl LayoutComponent<Html, Extractions> for MainPageBanner {
    fn matches(&self, content: &Html) -> bool {
        let selector = Selector::parse(".main-page-tag-lcs").unwrap();
        content.select(&selector).next().is_some()
    }

    fn extract(&self, _request: &SimpleRequest, _content: &Html) -> Vec<Extractions> {
        Vec::new()
    }
}

#[derive(Debug)]
struct StoryArticle;
impl LayoutComponent<Html, Extractions> for StoryArticle {
    fn matches(&self, content: &Html) -> bool {
        let selector = Selector::parse("#infoboxinternal").unwrap();
        content.select(&selector).next().is_some()
    }

    fn extract(&self, _request: &SimpleRequest, _content: &Html) -> Vec<Extractions> {
        Vec::new()
    }
}
#[derive(Debug)]
struct ChapterSumary;
impl LayoutComponent<Html, Extractions> for ChapterSumary {
    fn matches(&self, content: &Html) -> bool {
        let selector = Selector::parse("td").unwrap();
        content
            .select(&selector)
            .find(|section| {
                section
                    .text()
                    .collect::<String>()
                    .to_lowercase()
                    .contains("chapter guide")
            })
            .is_some()
    }

    fn extract(&self, _request: &SimpleRequest, _content: &Html) -> Vec<Extractions> {
        Vec::new()
    }
}
#[derive(Debug)]
struct ArcSummary;
impl LayoutComponent<Html, Extractions> for ArcSummary {
    fn matches(&self, content: &Html) -> bool {
        let selector = Selector::parse("td").unwrap();
        content
            .select(&selector)
            .find(|section| {
                section
                    .text()
                    .collect::<String>()
                    .to_lowercase()
                    .contains("arc guide")
            })
            .is_some()
    }

    fn extract(&self, _request: &SimpleRequest, _content: &Html) -> Vec<Extractions> {
        Vec::new()
    }
}
#[derive(Debug)]
struct CategoryPage;
impl LayoutComponent<Html, Extractions> for CategoryPage {
    fn matches(&self, content: &Html) -> bool {
        let selector = Selector::parse(".page-header__page-subtitle").unwrap();
        content
            .select(&selector)
            .next()
            .is_some_and(|section| section.text().collect::<String>().contains("Category page"))
    }

    fn extract(&self, _request: &SimpleRequest, _content: &Html) -> Vec<Extractions> {
        Vec::new()
    }
}

#[cfg(test)]
mod test {
    use super::clean_names;

    #[test]
    fn test_clean_names() {
        let cleaned = clean_names(vec![
            "Victoria Dallon".into(),
            "Vicky".into(),
            "Glory Girl".into(),
            "Antares".into(),
            "Point_Me_@_The_Sky (civilian ".into(),
            "PHO".into(),
            " handle)".into(),
            "Alexandria Junior (By ".into(),
            "Taylor".into(),
            ")".into(),
            "[1]".into(),
            "Glory Hole (By ".into(),
            "Tattletale".into(),
            ")".into(),
            "[1]".into(),
            "[2]".into(),
            "[3]".into(),
            "Big V (By ".into(),
            "Vista".into(),
            ")".into(),
            "[4]".into(),
            "[5]".into(),
            "[6]".into(),
        ]);
        let expected: Vec<String> = vec![
            "Victoria Dallon".into(),
            "Vicky".into(),
            "Glory Girl".into(),
            "Antares".into(),
            "Point_Me_@_The_Sky".into(),
            "Alexandria Junior".into(),
            "Glory Hole".into(),
            "Big V".into(),
        ];
        assert_eq!(cleaned, expected);
    }
}
