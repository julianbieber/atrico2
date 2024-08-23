use std::{collections::HashSet, path::PathBuf, sync::Arc, time::Duration};

use reqwest::Url;
use tokio::{spawn, task::JoinHandle, time::sleep};
use tracing::warn;

use crate::{
    parser::Parser,
    requester::{Requester, SimpleRequest},
};

pub struct Spider {
    state: SpiderState,
    requester: Arc<Requester>,
    open_requests: Vec<JoinHandle<Vec<SimpleRequest>>>,
}

pub trait RequestFilter {
    fn is_valid(&self, request: &SimpleRequest) -> bool;
}

impl Spider {
    pub async fn run<P, R>(
        initial: Vec<SimpleRequest>,
        parser: P,
        request_filter: R,
        cache_dir: PathBuf,
    ) where
        P: Parser + Clone + Send + 'static,
        R: RequestFilter,
    {
        let s = Spider {
            state: SpiderState::new(initial),
            open_requests: Vec::new(),
            requester: Arc::new(Requester::new(cache_dir)),
        };
        s.run_internal(parser, request_filter).await
    }
    async fn run_internal<P, R>(mut self, parser: P, request_filter: R)
    where
        P: Parser + Clone + Send + 'static,
        R: RequestFilter,
    {
        loop {
            let mut stepped = false;
            while let Some(r) = self.state.next() {
                stepped = true;
                let req = self.requester.clone();
                let p = parser.clone();
                self.open_requests.push(spawn(async move {
                    let response = req.execute(r.clone()).await;
                    p.parse(&r, &response).await
                }));
            }
            let mut new_jobs = Vec::new();
            for job in self.open_requests.into_iter() {
                if job.is_finished() {
                    stepped = true;
                    match job.await {
                        Ok(new_requests) => {
                            for r in new_requests.iter() {
                                if request_filter.is_valid(r) {
                                    self.state.add(r.clone());
                                }
                            }
                        }
                        Err(_e) => {
                            // warn!(error =? e, "failure during request");
                        }
                    }
                } else {
                    new_jobs.push(job);
                }
            }
            self.open_requests = new_jobs;

            // dbg!(self.open_requests.len() + self.state.open.len());

            if self.open_requests.is_empty() && self.state.is_empty() {
                break;
            } else {
                if !stepped {
                    sleep(Duration::from_millis(10)).await;
                }
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
