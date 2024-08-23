use std::{fmt::Debug, future::Future, sync::Arc};

use tracing::warn;

use crate::requester::SimpleRequest;

pub trait LayoutComponent<Content, Extracted>: Debug {
    fn matches(&self, content: &Content) -> bool;
    fn extract(&self, request: &SimpleRequest, content: &Content) -> Vec<Extracted>;
    fn name(&self) -> String {
        format!("{self:?}")
    }
}

pub struct Layout<Content, Extracted> {
    pub components: Vec<Box<dyn LayoutComponent<Content, Extracted> + Sync + Send + 'static>>,
}

impl<Content, Extracted> Layout<Content, Extracted> {
    fn name(&self) -> String {
        self.components
            .iter()
            .flat_map(|c| [c.name(), ", ".into()])
            .collect()
    }
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
                let components_names: String = matching
                    .iter()
                    .flat_map(|l| [l.name(), "\n".into()])
                    .collect();
                warn!(request = ?request, layout = ?components_names,  "too many matching layouts");
                panic!("too many matching layouts for {request:?}, matching layouts: {components_names}");
            }
            if matching.len() == 0 {
                warn!(request = ?request, "No matching layout");
                panic!("No matching layout for {request:?}");
            }
            let layout = matching[0];
            layout.extract(request, &content)
        };
        router(extracted).await
    }
}
