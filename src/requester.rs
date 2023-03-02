use std::{path::PathBuf, sync::Arc};

use reqwest::Client;
use reqwest::{Request, Url};

pub struct Requester {
    cache: Mutex<PageCache>,
    client: Client,
}

impl Requester {
    pub fn new(cache_dir: PathBuf) -> Requester {
        Requester {
            cache: Mutex::new(PageCache {
                base_dir: cache_dir,
            }),
            client: Client::new(),
        }
    }
    pub async fn execute(self: Arc<Self>, r: Request) -> String {
        if let Some(r) = self.get_from_cache(r.url()).await {
            r
        } else {
            let u = r.url().clone();
            let response = self.client.execute(r).await.unwrap();
            let content = response.text().await.unwrap();
            self.write_to_cache(&u, &content).await;
            content
        }
    }

    async fn get_from_cache(&self, url: &Url) -> Option<String> {
        let c = self.cache.lock().await;
        c.get(url).await
    }

    async fn write_to_cache(&self, url: &Url, content: &str) {
        let c = self.cache.lock().await;
        c.add(url, content).await;
    }
}

use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

struct PageCache {
    base_dir: PathBuf,
}

impl PageCache {
    async fn add(&self, url: &Url, content: &str) {
        let path = self
            .base_dir
            .join(urlencoding::encode(url.as_str()).as_ref());
        let mut file = File::create(path).await.unwrap();
        file.write_all(content.as_bytes()).await.unwrap();
        file.flush().await.unwrap();
    }

    async fn get(&self, url: &Url) -> Option<String> {
        let path = self
            .base_dir
            .join(urlencoding::encode(url.as_str()).as_ref());
        let mut file = File::open(path).await.ok()?;
        let mut s = String::new();
        file.read_to_string(&mut s).await.unwrap();
        Some(s)
    }
}
