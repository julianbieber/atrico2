use scraper::Html;

pub fn parse(string: &str) -> Html {
    Html::parse_document(string)
}
