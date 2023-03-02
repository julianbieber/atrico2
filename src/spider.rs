use std::{collections::HashSet, path::PathBuf, sync::Arc, time::Duration};

use reqwest::Url;
use tokio::{spawn, task::JoinHandle, time::sleep};

use crate::{
    parser::Parser,
    requester::{Requester, SimpleRequest},
};

pub struct Spider {
    state: SpiderState,
    requester: Arc<Requester>,
    open_requests: Vec<JoinHandle<Vec<SimpleRequest>>>,
}

impl Spider {
    pub async fn run<P>(initial: Vec<SimpleRequest>, parser: P, cache_dir: PathBuf)
    where
        P: Parser + Clone + Send + 'static,
    {
        let s = Spider {
            state: SpiderState::new(initial),
            open_requests: Vec::new(),
            requester: Arc::new(Requester::new(cache_dir)),
        };
        s.run_internal(parser).await
    }
    async fn run_internal<P>(mut self, parser: P)
    where
        P: Parser + Clone + Send + 'static,
    {
        loop {
            if let Some(r) = self.state.next() {
                let u = r.url.as_str();
                dbg!(u);
                let req = self.requester.clone();
                let p = parser.clone();
                self.open_requests.push(spawn(async move {
                    let response = req.execute(r).await;
                    p.parse(&response).await
                }));
            }
            let mut new_jobs = Vec::new();
            for job in self.open_requests.into_iter() {
                if job.is_finished() {
                    if let Ok(new_requests) = job.await {
                        for r in new_requests.iter() {
                            self.state.add(r.clone());
                        }
                    }
                } else {
                    new_jobs.push(job);
                }
            }
            self.open_requests = new_jobs;

            if self.open_requests.is_empty() && self.state.is_empty() {
                break;
            } else {
                sleep(Duration::from_millis(100)).await;
            }
        }
    }
}
struct SpiderState {
    open: Vec<SimpleRequest>,
    seen: HashSet<Url>,
}

impl SpiderState {
    fn new(initial_requests: Vec<SimpleRequest>) -> SpiderState {
        let mut s = SpiderState {
            open: Vec::new(),
            seen: HashSet::new(),
        };
        for r in initial_requests {
            s.add(r);
        }
        s
    }

    fn add(&mut self, r: SimpleRequest) {
        if self.seen.insert(r.url.clone()) {
            self.open.push(r);
        }
    }

    fn next(&mut self) -> Option<SimpleRequest> {
        self.open.pop()
    }

    fn is_empty(&self) -> bool {
        self.open.is_empty()
    }
}
