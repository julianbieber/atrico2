use std::sync::Arc;

use reqwest::Request;

pub struct Requester {}


impl Requester {
    pub async fn execute(self: Arc<Self>, r: Request) -> String {
        String::new()
    }
}