use std::{future::Future, sync::Arc};

use crate::requester::SimpleRequest;

pub trait LayoutComponent<Content, Extracted> {
    fn matches(&self, content: &Content) -> bool;
    fn extract(&self, request: &SimpleRequest, content: &Content) -> Vec<Extracted>;
}

pub struct Layout<Content, Extracted> {
    pub components: Vec<Box<dyn LayoutComponent<Content, Extracted> + Sync + Send + 'static>>,
}

impl<Content, Extracted> Layout<Content, Extracted> {
    fn extract(&self, request: &SimpleRequest, content: &Content) -> Vec<Extracted> {
        self.components
            .iter()
            .flat_map(|c| c.extract(request, &content))
            .collect()
    }
    fn matches(&self, content: &Content) -> bool {
        self.components.iter().all(|c| c.matches(&content))
    }
}

#[derive(Clone)]
pub struct LayoutParser<Content, Extracted> {
    pub layouts: Arc<Vec<Layout<Content, Extracted>>>,
}
impl<Content, Extracted> LayoutParser<Content, Extracted> {
    pub async fn parse<F, Router, Fut>(
        self,
        request: &SimpleRequest,
        page: &str,
        parser: F,
        router: Router,
    ) -> Vec<SimpleRequest>
    where
        F: Fn(&str) -> Content,
        Router: FnOnce(Vec<Extracted>) -> Fut,
        Fut: Future<Output = Vec<SimpleRequest>>,
    {
        let extracted = {
            let content = parser(page);
            let matching: Vec<_> = self
                .layouts
                .iter()
                .filter(|l| l.matches(&content))
                .collect();
            if matching.len() > 1 {
                panic!("too many matching layouts");
            }
            if matching.len() == 0 {
                panic!("No matching layout");
            }
            let layout = matching[0];
            layout.extract(request, &content)
        };
        router(extracted).await
    }
}
