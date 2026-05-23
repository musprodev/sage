use std::sync::Arc;
use tokio;
mod scraper;
mod models;
mod error;

#[tokio::main]
async fn main() {
    let provider = scraper::NovelBuddy::new();
    // we need to find a chapter url first
}
