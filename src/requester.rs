use std::time::Duration;
use std::{path::PathBuf, sync::Arc};

use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use reqwest::header::HeaderMap;
use reqwest::{Body, Client, Method};
use reqwest::{Request, Url};

#[derive(Clone, Debug)]
pub struct SimpleRequest {
    pub method: Method,
    pub url: Url,
    pub headers: HeaderMap,
    pub body: Option<String>,
}

impl From<SimpleRequest> for Request {
    fn from(value: SimpleRequest) -> Self {
        let mut r = Request::new(value.method, value.url);
        (*r.headers_mut()) = value.headers;
        (*r.body_mut()) = value.body.map(Body::from);
        r
    }
}

pub struct Requester {
    cache: Mutex<PageCache>,
    clients: ClientProvider,
}

impl Requester {
    pub fn new(cache_dir: PathBuf) -> Requester {
        Requester {
            cache: Mutex::new(PageCache {
                base_dir: cache_dir,
            }),
            clients: ClientProvider::new(),
        }
    }
    pub async fn execute(self: Arc<Self>, r: SimpleRequest) -> String {
        if let Some(r) = self.get_from_cache(&r.url).await {
            r
        } else {
            let u = r.url.clone();
            let client = self.clients.get_client().await;
            let content = Self::e(&client, r).await;
            self.clients.return_client(client);
            let content = content.unwrap();
            self.write_to_cache(&u, &content).await;
            content
        }
    }

    async fn e(client: &Client, r: SimpleRequest) -> Result<String, reqwest::Error> {
        let response = client.execute(r.into()).await?;
        let content = response.text().await?;
        Ok(content)
    }

    async fn get_from_cache(&self, url: &Url) -> Option<String> {
        let c = self.cache.lock().await;
        let compressed = c.get(url).await?;
        drop(c);
        let mut decoder = ZlibDecoder::new(compressed.as_slice());

        let mut decompressed = String::new();
        decoder.read_to_string(&mut decompressed).unwrap();
        Some(decompressed)
    }

    async fn write_to_cache(&self, url: &Url, content: &str) {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(content.as_bytes()).unwrap();
        let compressed = encoder.finish().unwrap();
        let c = self.cache.lock().await;
        c.add(url, &compressed).await;
    }
}

use std::io::prelude::*;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::spawn;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::Mutex;
use tokio::time::sleep;

struct PageCache {
    base_dir: PathBuf,
}

impl PageCache {
    async fn add(&self, url: &Url, content: &[u8]) {
        let path = self
            .base_dir
            .join(urlencoding::encode(url.as_str()).as_ref());
        let mut file = File::create(path).await.unwrap();
        file.write_all(&content).await.unwrap();
        file.flush().await.unwrap();
    }

    async fn get(&self, url: &Url) -> Option<Vec<u8>> {
        let path = self
            .base_dir
            .join(urlencoding::encode(url.as_str()).as_ref());
        let mut file = File::open(path).await.ok()?;
        let mut s = Vec::new();
        file.read_to_end(&mut s).await.unwrap();

        Some(s)
    }
}

struct ClientProvider {
    all_clients: Mutex<UnboundedReceiver<Client>>,
    returned_clients: UnboundedSender<Client>,
}

impl ClientProvider {
    fn new() -> ClientProvider {
        let (sender, reciever) = unbounded_channel();
        sender
            .send(
                Client::builder()
                    .cookie_store(true)
                    .user_agent(
                        "Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/110.0",
                    )
                    .build()
                    .unwrap(),
            )
            .unwrap();
        ClientProvider {
            all_clients: Mutex::new(reciever),
            returned_clients: sender,
        }
    }

    async fn get_client(&self) -> Client {
        let mut r = self.all_clients.lock().await;
        r.recv().await.unwrap()
    }

    fn return_client(&self, c: Client) {
        let r = self.returned_clients.clone();
        spawn(async move {
            sleep(Duration::from_millis(1000)).await;
            r.send(c).unwrap()
        });
    }
}
