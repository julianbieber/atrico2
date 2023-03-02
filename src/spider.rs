use std::{collections::HashSet, time::Duration, sync::Arc};

use reqwest::{Request, Url};
use tokio::{spawn, task::JoinHandle, time::sleep};

use crate::{parser::Parser, requester::Requester};

pub struct Spider {
    state: SpiderState,
    requester: Arc<Requester>,
    open_requests: Vec<JoinHandle<Vec<Request>>>,
}

impl Spider {
    pub async fn run<P>(initial: Vec<Request>, parser: P) where P: Parser + Copy + Send + 'static{
        let s = Spider {
            state: SpiderState::new(initial),
            open_requests: Vec::new(),
            requester: Arc::new(Requester {}),
        };
        s.run_internal(parser).await
    }
    async fn run_internal<P>(mut self, parser: P) where P: Parser + Copy + Send + 'static {
        loop {
            if let Some(r) = self.state.next() {
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
                        for r in new_requests {
                            self.state.add(r);
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
    open: Vec<Request>,
    seen: HashSet<Url>,
}

impl SpiderState {
    fn new(initial_requests: Vec<Request>) -> SpiderState {
        let mut s = SpiderState {
            open: Vec::new(),
            seen: HashSet::new(),
        };
        for r in initial_requests {
            s.add(r);
        }
        s
    }

    fn add(&mut self, r: Request) {
        if self.seen.insert(r.url().clone()) {
            self.open.push(r);
        }
    }

    fn next(&mut self) -> Option<Request> {
        self.open.pop()
    }

    fn is_empty(&self) -> bool {
        self.open.is_empty()
    }
}
